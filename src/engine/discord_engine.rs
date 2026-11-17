// Talks to Discord's REST API (twilight-http): registers slash commands and answers
// interactions. The live connection (gateway) is in handlers/discord_handler.rs.
use crate::domain::discord::{DiscordButton, DiscordCommand, DiscordInteraction, InteractionKind};
use anyhow::Result;
#[cfg(test)]
use mockall::automock;
use twilight_http::Client;
use twilight_model::application::command::CommandType;
use twilight_model::application::interaction::{Interaction, InteractionData};
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_util::builder::command::CommandBuilder;

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
    // Answers a button click by editing the message it belongs to (its buttons are removed).
    async fn update_message(&self, interaction_id: u64, token: String, text: String) -> Result<()>;
}

pub struct DiscordApi {
    http: Client,
    application_id: Id<ApplicationMarker>,
}

impl DiscordApi {
    // The application id comes from the gateway's Ready event.
    pub fn new(token: String, application_id: Id<ApplicationMarker>) -> Self {
        DiscordApi {
            http: Client::new(token),
            application_id,
        }
    }
}

#[async_trait::async_trait]
impl DiscordExecutor for DiscordApi {
    async fn register_commands(
        &self,
        guild_id: Option<u64>,
        commands: Vec<DiscordCommand>,
    ) -> Result<usize> {
        let commands: Vec<_> = commands
            .iter()
            .map(|command| {
                CommandBuilder::new(&command.name, &command.description, CommandType::ChatInput)
                    .build()
            })
            .collect();
        let client = self.http.interaction(self.application_id);
        match guild_id {
            Some(guild_id) => {
                client
                    .set_guild_commands(Id::new(guild_id), &commands)
                    .await?;
            }
            None => {
                client.set_global_commands(&commands).await?;
            }
        }
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
            components: (!buttons.is_empty()).then(|| vec![button_row(buttons)]),
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

    async fn update_message(&self, interaction_id: u64, token: String, text: String) -> Result<()> {
        let data = InteractionResponseData {
            content: Some(text),
            // An empty list removes the buttons.
            components: Some(Vec::new()),
            ..Default::default()
        };
        let response = InteractionResponse {
            kind: InteractionResponseType::UpdateMessage,
            data: Some(data),
        };
        self.http
            .interaction(self.application_id)
            .create_response(Id::new(interaction_id), &token, &response)
            .await?;
        Ok(())
    }
}

// Our buttons -> one Discord row of blue buttons.
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

// Twilight's interaction -> ours. None for things the bot doesn't handle (modals, pings...).
pub fn to_discord_interaction(interaction: &Interaction) -> Option<DiscordInteraction> {
    let kind = match interaction.data.as_ref()? {
        InteractionData::ApplicationCommand(data) => InteractionKind::Command(data.name.clone()),
        InteractionData::MessageComponent(data) => InteractionKind::Button(data.custom_id.clone()),
        _ => return None,
    };
    Some(DiscordInteraction {
        id: interaction.id.get(),
        token: interaction.token.clone(),
        user_id: interaction.author_id()?.get(),
        kind,
    })
}
