// Talks to Discord's REST API (twilight-http): registers slash commands, answers
// interactions and sends messages. The live connection (gateway) is in
// handlers/discord_handler.rs.
use crate::domain::discord::{
    Defer, DiscordButton, DiscordCommand, DiscordInteraction, InteractionKind,
};
use anyhow::{Result, anyhow};
#[cfg(test)]
use mockall::automock;
use std::sync::Arc;
use twilight_http::Client;
use twilight_http::api_error::ApiError;
use twilight_http::error::ErrorType;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::{
    CommandDataOption, CommandOptionValue,
};
use twilight_model::application::interaction::{Interaction, InteractionData};
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

// Handlers use this trait instead of DiscordApi directly, so tests can pass a mock.
#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait DiscordExecutor: Send + Sync {
    // Replaces the bot's slash commands: in one server (shows up at once, good for
    // development) or everywhere (`guild_id` None). Returns how many were registered.
    async fn register_commands(
        &self,
        guild_id: Option<u64>,
        commands: Vec<DiscordCommand>,
    ) -> Result<usize>;
    // Answers with a new message. `private` = only the user who acted sees it.
    async fn reply(
        &self,
        interaction_id: u64,
        token: String,
        text: String,
        buttons: Vec<DiscordButton>,
        private: bool,
    ) -> Result<()>;
    // Answers a button click by editing the message it belongs to. EX0039: `buttons`
    // replace the message's buttons; an empty list removes them.
    async fn update_message(
        &self,
        interaction_id: u64,
        token: String,
        text: String,
        buttons: Vec<DiscordButton>,
    ) -> Result<()>;
    // EX0036: answers "I'm working on it" within Discord's 3 seconds; the real answer then
    // comes with follow_up (the token stays valid for 15 minutes).
    async fn defer(&self, interaction_id: u64, token: String, defer: Defer) -> Result<()>;
    // EX0036: a message after defer (or a second message after a reply).
    async fn follow_up(
        &self,
        token: String,
        text: String,
        buttons: Vec<DiscordButton>,
        private: bool,
    ) -> Result<()>;
}

// EX0036: what an app plugs into the Discord bot (utils/pack_discord_app.rs picks one).
// Each interaction runs in its own task, so `handle` may take its time (defer first if
// it takes more than ~2 seconds).
#[async_trait::async_trait]
pub trait DiscordApp: Send + Sync {
    // The slash commands to register when the bot connects.
    fn commands(&self) -> Vec<DiscordCommand>;
    async fn handle(
        &self,
        api: Arc<dyn DiscordExecutor>,
        interaction: DiscordInteraction,
    ) -> Result<()>;
}

pub struct DiscordApi {
    http: Arc<Client>,
    application_id: Id<ApplicationMarker>,
}

impl DiscordApi {
    // The application id comes from the gateway's Ready event. `http` is shared with
    // DiscordSender, so both use the same rate limits.
    pub fn new(http: Arc<Client>, application_id: Id<ApplicationMarker>) -> Self {
        DiscordApi {
            http,
            application_id,
        }
    }
}

// Our command -> Discord's.
pub fn to_twilight_command(command: &DiscordCommand) -> Command {
    let mut builder =
        CommandBuilder::new(&command.name, &command.description, CommandType::ChatInput);
    for option in &command.options {
        let mut text =
            StringBuilder::new(&option.name, &option.description).required(option.required);
        if !option.choices.is_empty() {
            text = text.choices(option.choices.iter().map(|c| (c.clone(), c.clone())));
        }
        builder = builder.option(text);
    }
    if command.admin_only {
        // EX0039: "0" = no permission is enough, so only administrators see it.
        builder = builder.default_member_permissions(Permissions::empty());
    }
    builder.build()
}

// EX0036: Discord's 403 "Missing Access" (50001) when registering commands in a server
// says nothing useful; this says what to do. Other errors pass through unchanged.
pub fn explain_register_error(err: twilight_http::Error, guild_id: Option<u64>) -> anyhow::Error {
    let missing_access = matches!(
        err.kind(),
        ErrorType::Response { error: ApiError::General(general), .. } if general.code == 50001
    );
    match (missing_access, guild_id) {
        (true, Some(guild_id)) => anyhow!(
            "Discord refused to register the commands in server {guild_id} (Missing Access, 50001): \
             invite the bot to that server with the `bot` and `applications.commands` scopes, \
             or unset DISCORD_GUILD_ID to register them globally"
        ),
        _ => err.into(),
    }
}

#[async_trait::async_trait]
impl DiscordExecutor for DiscordApi {
    async fn register_commands(
        &self,
        guild_id: Option<u64>,
        commands: Vec<DiscordCommand>,
    ) -> Result<usize> {
        let commands: Vec<_> = commands.iter().map(to_twilight_command).collect();
        let client = self.http.interaction(self.application_id);
        let result = match guild_id {
            Some(guild_id) => client
                .set_guild_commands(Id::new(guild_id), &commands)
                .await
                .map(|_| ()),
            None => client.set_global_commands(&commands).await.map(|_| ()),
        };
        result.map_err(|err| explain_register_error(err, guild_id))?;
        Ok(commands.len())
    }

    async fn reply(
        &self,
        interaction_id: u64,
        token: String,
        text: String,
        buttons: Vec<DiscordButton>,
        private: bool,
    ) -> Result<()> {
        let data = InteractionResponseData {
            content: Some(text),
            components: (!buttons.is_empty()).then(|| button_rows(buttons)),
            flags: private.then_some(MessageFlags::EPHEMERAL),
            ..Default::default()
        };
        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(data),
        };
        self.http
            .interaction(self.application_id)
            .create_response(Id::new(interaction_id), &token, &response)
            .await?;
        Ok(())
    }

    async fn update_message(
        &self,
        interaction_id: u64,
        token: String,
        text: String,
        buttons: Vec<DiscordButton>,
    ) -> Result<()> {
        let response = update_response(text, buttons);
        self.http
            .interaction(self.application_id)
            .create_response(Id::new(interaction_id), &token, &response)
            .await?;
        Ok(())
    }

    async fn defer(&self, interaction_id: u64, token: String, defer: Defer) -> Result<()> {
        let response = defer_response(defer);
        self.http
            .interaction(self.application_id)
            .create_response(Id::new(interaction_id), &token, &response)
            .await?;
        Ok(())
    }

    async fn follow_up(
        &self,
        token: String,
        text: String,
        buttons: Vec<DiscordButton>,
        private: bool,
    ) -> Result<()> {
        let components = button_rows(buttons);
        let client = self.http.interaction(self.application_id);
        let mut request = client
            .create_followup(&token)
            .content(&text)
            .components(&components);
        if private {
            request = request.flags(MessageFlags::EPHEMERAL);
        }
        request.await?;
        Ok(())
    }
}

// EX0039: the response that edits a clicked message: new text, new buttons (an empty
// list removes them, rather than keeping the old ones).
pub fn update_response(text: String, buttons: Vec<DiscordButton>) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::UpdateMessage,
        data: Some(InteractionResponseData {
            content: Some(text),
            components: Some(button_rows(buttons)),
            ..Default::default()
        }),
    }
}

// The response that defers: type 5 for a command, type 6 for a button.
pub fn defer_response(defer: Defer) -> InteractionResponse {
    match defer {
        Defer::Reply { private } => InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: private.then(|| InteractionResponseData {
                flags: Some(MessageFlags::EPHEMERAL),
                ..Default::default()
            }),
        },
        Defer::Update => InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: None,
        },
    }
}

// EX0036: sends messages that aren't answers to an interaction (a DM to a player, a post
// in a channel), from anywhere in the app: AppState.discord holds one.
#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait DiscordMessenger: Send + Sync {
    // A direct message. Fails with Discord's 50007 when the user doesn't accept DMs from
    // the bot (no shared server, or DMs turned off).
    async fn send_dm(&self, user_id: u64, text: String, buttons: Vec<DiscordButton>) -> Result<()>;
    async fn send_to_channel(
        &self,
        channel_id: u64,
        text: String,
        buttons: Vec<DiscordButton>,
    ) -> Result<()>;
}

pub struct DiscordSender {
    http: Arc<Client>,
}

impl DiscordSender {
    pub fn new(http: Arc<Client>) -> Self {
        DiscordSender { http }
    }

    // The HTTP client, shared with the bot worker (same rate limits).
    pub fn http(&self) -> Arc<Client> {
        self.http.clone()
    }

    // For AppState: None without DISCORD_TOKEN (the bot worker then stops the app anyway).
    // DISCORD_API_PROXY (tests only) sends the REST calls to a fake server over plain HTTP.
    pub fn from_env() -> Option<Arc<DiscordSender>> {
        let token = std::env::var("DISCORD_TOKEN").ok()?;
        if token.trim().is_empty() {
            return None;
        }
        let mut builder = Client::builder().token(token);
        if let Ok(proxy) = std::env::var("DISCORD_API_PROXY") {
            builder = builder.proxy(proxy, true).ratelimiter(None);
        }
        Some(Arc::new(DiscordSender::new(Arc::new(builder.build()))))
    }
}

#[async_trait::async_trait]
impl DiscordMessenger for DiscordSender {
    async fn send_dm(&self, user_id: u64, text: String, buttons: Vec<DiscordButton>) -> Result<()> {
        let channel = self
            .http
            .create_private_channel(Id::new(user_id))
            .await?
            .model()
            .await?;
        self.send_to_channel(channel.id.get(), text, buttons).await
    }

    async fn send_to_channel(
        &self,
        channel_id: u64,
        text: String,
        buttons: Vec<DiscordButton>,
    ) -> Result<()> {
        let components = button_rows(buttons);
        self.http
            .create_message(Id::new(channel_id))
            .content(&text)
            .components(&components)
            .await?;
        Ok(())
    }
}

// Discord's limits: 5 buttons per row, 5 rows per message.
pub const BUTTONS_PER_ROW: usize = 5;
pub const MAX_BUTTONS: usize = 25;

// Our buttons -> rows of blue buttons, 5 per row (EX0039; was one row, so a 6th button
// made Discord refuse the message). None when there are no buttons; past 25, the rest
// are dropped.
pub fn button_rows(buttons: Vec<DiscordButton>) -> Vec<Component> {
    let buttons: Vec<DiscordButton> = buttons.into_iter().take(MAX_BUTTONS).collect();
    buttons
        .chunks(BUTTONS_PER_ROW)
        .map(|row| button_row(row.to_vec()))
        .collect()
}

fn button_row(buttons: Vec<DiscordButton>) -> Component {
    Component::ActionRow(ActionRow {
        components: buttons
            .into_iter()
            .map(|button| {
                Component::Button(Button {
                    custom_id: Some(button.custom_id),
                    disabled: false,
                    emoji: None,
                    id: None,
                    label: Some(button.label),
                    sku_id: None,
                    style: ButtonStyle::Primary,
                    url: None,
                })
            })
            .collect(),
        id: None,
    })
}

// A command's options as name/value text. A subcommand adds its own name (empty value)
// followed by its options.
pub fn option_values(options: &[CommandDataOption]) -> Vec<(String, String)> {
    let mut values = Vec::new();
    for option in options {
        let value = match &option.value {
            CommandOptionValue::String(text) => text.clone(),
            CommandOptionValue::Integer(number) => number.to_string(),
            CommandOptionValue::Number(number) => number.to_string(),
            CommandOptionValue::Boolean(flag) => flag.to_string(),
            CommandOptionValue::Focused(text, _) => text.clone(),
            CommandOptionValue::User(id) => id.to_string(),
            CommandOptionValue::Channel(id) => id.to_string(),
            CommandOptionValue::Role(id) => id.to_string(),
            CommandOptionValue::Mentionable(id) => id.to_string(),
            CommandOptionValue::Attachment(id) => id.to_string(),
            CommandOptionValue::SubCommand(inner) | CommandOptionValue::SubCommandGroup(inner) => {
                values.push((option.name.clone(), String::new()));
                values.extend(option_values(inner));
                continue;
            }
        };
        values.push((option.name.clone(), value));
    }
    values
}

// Twilight's interaction -> ours. None for things the bot doesn't handle (modals, pings...).
pub fn to_discord_interaction(interaction: &Interaction) -> Option<DiscordInteraction> {
    let (kind, options) = match interaction.data.as_ref()? {
        InteractionData::ApplicationCommand(data) => (
            InteractionKind::Command(data.name.clone()),
            option_values(&data.options),
        ),
        InteractionData::MessageComponent(data) => {
            (InteractionKind::Button(data.custom_id.clone()), Vec::new())
        }
        _ => return None,
    };
    // In a server the user comes inside `member` (with the server nickname); in a DM, as `user`.
    let user = interaction
        .member
        .as_ref()
        .and_then(|member| member.user.as_ref())
        .or(interaction.user.as_ref());
    let nickname = interaction
        .member
        .as_ref()
        .and_then(|member| member.nick.clone());
    Some(DiscordInteraction {
        id: interaction.id.get(),
        token: interaction.token.clone(),
        user_id: interaction.author_id()?.get(),
        user_name: user.map(|user| user.name.clone()),
        display_name: nickname
            .or_else(|| user.and_then(|user| user.global_name.clone()))
            .or_else(|| user.map(|user| user.name.clone())),
        kind,
        locale: interaction.locale.clone(),
        guild_id: interaction.guild_id.map(|id| id.get()),
        options,
    })
}
