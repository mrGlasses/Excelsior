// The Discord bot: a background worker holding the gateway connection (twilight-gateway).
// Each slash command or button click is handed to the app (utils/pack_discord_app.rs picks
// it; DemoDiscordApp below by default) in its own task, so a slow answer never delays the
// next user (EX0036).
use crate::domain::discord::{
    Defer, DiscordButton, DiscordCommand, DiscordInteraction, DiscordOption, InteractionKind,
};
use crate::engine::discord_engine::{
    DiscordApi, DiscordApp, DiscordExecutor, to_discord_interaction,
};
use crate::state::AppState;
use crate::utils::workers::Workers;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use twilight_gateway::{
    CloseFrame, ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client;
use twilight_model::user::CurrentUser;

// After asking the gateway to close, wait at most this long for it to finish.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(3);
// On shutdown, interactions still running get this long to finish, then they're dropped.
const IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(3);
// Used when DISCORD_MAX_CONCURRENT is not set: interactions handled at the same time.
pub const DEFAULT_MAX_CONCURRENT: usize = 32;
// How long the demo /slow command works before answering.
pub const SLOW_DEMO_DELAY: Duration = Duration::from_secs(5);

pub struct DiscordSettings {
    pub token: String,
    // Register commands in this server only (instant, for development); None = everywhere.
    pub guild_id: Option<u64>,
    // Only for tests: connect to this address instead of Discord's gateway.
    pub gateway_url: Option<String>,
    // EX0036: at most this many interactions are handled at the same time.
    pub max_concurrent: usize,
}

// DISCORD_GUILD_ID: unset or empty = register commands everywhere.
pub fn discord_guild_id_from(value: Option<String>) -> Option<u64> {
    let text = value?;
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    match text.parse::<u64>() {
        Ok(id) if id > 0 => Some(id),
        _ => panic!("DISCORD_GUILD_ID must be a server id (a number)."),
    }
}

// DISCORD_MAX_CONCURRENT: unset = the default; otherwise a number greater than 0.
pub fn max_concurrent_from(value: Option<String>) -> usize {
    match value {
        None => DEFAULT_MAX_CONCURRENT,
        Some(text) => match text.trim().parse::<usize>() {
            Ok(number) if number > 0 => number,
            _ => panic!("DISCORD_MAX_CONCURRENT must be a number greater than 0."),
        },
    }
}

// Called from utils/pack_workers.rs. False (the app exits with code 1) without a token.
pub fn start(workers: &mut Workers, state: &AppState) -> bool {
    let token = match std::env::var("DISCORD_TOKEN") {
        Ok(token) if !token.trim().is_empty() => token,
        _ => {
            error!("DISCORD_TOKEN is missing");
            return false;
        }
    };
    // The same HTTP client as AppState.discord (one set of rate limits for the whole app).
    let http = match &state.discord {
        Some(sender) => sender.http(),
        None => Arc::new(Client::new(token.clone())),
    };
    let app = crate::utils::pack_discord_app::discord_app(state);
    let settings = DiscordSettings {
        token,
        guild_id: discord_guild_id_from(std::env::var("DISCORD_GUILD_ID").ok()),
        gateway_url: std::env::var("DISCORD_GATEWAY_URL").ok(),
        max_concurrent: max_concurrent_from(std::env::var("DISCORD_MAX_CONCURRENT").ok()),
    };
    workers.spawn("discord", move |shutdown| {
        run_discord_bot(settings, app, http, shutdown)
    });
    true
}

// EX0036: the interactions being handled right now, each in its own task.
pub struct InteractionTasks {
    tasks: JoinSet<()>,
    permits: Arc<Semaphore>,
}

impl InteractionTasks {
    pub fn new(max_concurrent: usize) -> Self {
        InteractionTasks {
            tasks: JoinSet::new(),
            permits: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    // Starts handling one interaction. Waits only when max_concurrent are already running.
    pub async fn spawn(
        &mut self,
        app: Arc<dyn DiscordApp>,
        api: Arc<dyn DiscordExecutor>,
        interaction: DiscordInteraction,
    ) {
        // Forget the ones that already finished.
        while self.tasks.try_join_next().is_some() {}
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("the semaphore is never closed");
        self.tasks.spawn(async move {
            let _permit = permit;
            // A failing answer is logged and skipped: one bad click must not stop the bot.
            if let Err(err) = app.handle(api, interaction).await {
                warn!("discord interaction failed: {}", err);
            }
        });
    }

    pub fn running(&self) -> usize {
        self.tasks.len()
    }

    // On shutdown: let running interactions finish for up to `wait`, then drop the rest.
    pub async fn finish(mut self, wait: Duration) {
        let all_done = async { while self.tasks.join_next().await.is_some() {} };
        if tokio::time::timeout(wait, all_done).await.is_err() {
            warn!(
                "{} discord interactions still running at shutdown were dropped",
                self.tasks.len()
            );
            self.tasks.abort_all();
        }
    }
}

// The worker. Fails (and so stops the app) only when Discord closes the connection for
// good, e.g. a wrong token, or refuses the commands; network drops are reconnected by
// twilight by itself.
pub async fn run_discord_bot(
    settings: DiscordSettings,
    app: Arc<dyn DiscordApp>,
    http: Arc<Client>,
    shutdown: CancellationToken,
) -> Result<()> {
    // Twilight uses rustls for TLS but doesn't pick its crypto backend: without this line
    // the first connection panics. Err = another part of the app already installed one.
    let _ = rustls::crypto::ring::default_provider().install_default();
    // Slash commands and buttons don't need any privileged intents.
    let mut config = ConfigBuilder::new(settings.token.clone(), Intents::empty());
    if let Some(url) = settings.gateway_url.clone() {
        config = config.proxy_url(url);
    }
    let mut shard = Shard::with_config(ShardId::ONE, config.build());
    let wanted = EventTypeFlags::READY | EventTypeFlags::INTERACTION_CREATE;
    // Created on Ready: that's where we learn the application id.
    let mut api: Option<Arc<dyn DiscordExecutor>> = None;
    let mut tasks = InteractionTasks::new(settings.max_concurrent);

    loop {
        let next = tokio::select! {
            // `biased`: once shutdown is requested, stop taking new events.
            biased;
            _ = shutdown.cancelled() => break,
            next = shard.next_event(wanted) => next,
        };

        match next {
            // The shard ends its stream only when Discord refused us for good.
            None => {
                return Err(anyhow!(
                    "discord gateway closed for good (wrong DISCORD_TOKEN?)"
                ));
            }
            // Network hiccups: twilight reconnects by itself.
            Some(Err(err)) => warn!("discord gateway error: {}", err),
            Some(Ok(Event::Ready(ready))) => {
                info!("discord connected as {}", display_name(&ready.user));
                let discord = DiscordApi::new(http.clone(), ready.application.id);
                let registered = discord
                    .register_commands(settings.guild_id, app.commands())
                    .await?;
                match settings.guild_id {
                    Some(guild_id) => {
                        info!("registered {} commands in guild {}", registered, guild_id)
                    }
                    None => info!("registered {} commands globally", registered),
                }
                api = Some(Arc::new(discord));
            }
            Some(Ok(Event::InteractionCreate(event))) => {
                let (Some(api), Some(interaction)) = (&api, to_discord_interaction(&event.0))
                else {
                    continue;
                };
                tasks.spawn(app.clone(), api.clone(), interaction).await;
            }
            Some(Ok(_)) => {}
        }
    }

    tasks.finish(IN_FLIGHT_TIMEOUT).await;
    // Say goodbye properly, so the bot shows as offline soon instead of timing out.
    // (Close messages always come through next_event, whatever flags we pass.)
    shard.close(CloseFrame::NORMAL);
    let _ = tokio::time::timeout(CLOSE_TIMEOUT, async {
        while let Some(event) = shard.next_event(EventTypeFlags::empty()).await {
            if let Ok(Event::GatewayClose(_)) = event {
                break;
            }
        }
    })
    .await;
    Ok(())
}

// "CardBot#1234", or just "CardBot" for new-style usernames (discriminator 0).
pub fn display_name(user: &CurrentUser) -> String {
    if user.discriminator == 0 {
        user.name.clone()
    } else {
        format!("{}#{:04}", user.name, user.discriminator)
    }
}

// ---------------------------------------------------------------------------------------
// The demo app: what the bot does until an app plugs in its own (pack_discord_app.rs).
// ---------------------------------------------------------------------------------------

pub struct DemoDiscordApp;

#[async_trait::async_trait]
impl DiscordApp for DemoDiscordApp {
    fn commands(&self) -> Vec<DiscordCommand> {
        commands()
    }

    async fn handle(
        &self,
        api: Arc<dyn DiscordExecutor>,
        interaction: DiscordInteraction,
    ) -> Result<()> {
        handle_interaction(api.as_ref(), interaction).await
    }
}

// The demo's slash commands.
pub fn commands() -> Vec<DiscordCommand> {
    vec![
        DiscordCommand {
            name: "ping".to_string(),
            description: "Replies PONG!".to_string(),
            options: vec![],
            admin_only: false,
        },
        DiscordCommand {
            name: "choose".to_string(),
            description: "Pick Yes or No with buttons".to_string(),
            options: vec![DiscordOption {
                name: "question".to_string(),
                description: "What to choose about".to_string(),
                required: false,
                choices: vec![],
            }],
            admin_only: false,
        },
        DiscordCommand {
            name: "slow".to_string(),
            description: "Answers after 5 seconds (shows defer and follow-up)".to_string(),
            options: vec![],
            admin_only: false,
        },
    ]
}

pub async fn handle_interaction(
    api: &dyn DiscordExecutor,
    interaction: DiscordInteraction,
) -> Result<()> {
    match &interaction.kind {
        InteractionKind::Command(name) => handle_command(api, &interaction, name).await,
        InteractionKind::Button(custom_id) => handle_button(api, &interaction, custom_id).await,
    }
}

async fn handle_command(
    api: &dyn DiscordExecutor,
    interaction: &DiscordInteraction,
    name: &str,
) -> Result<()> {
    match name {
        "ping" => ping(api, interaction).await,
        "choose" => choose(api, interaction).await,
        "slow" => slow(api, interaction).await,
        // Only happens if Discord still lists an old command we no longer have.
        _ => {
            api.reply(
                interaction.id,
                interaction.token.clone(),
                "Unknown command.".to_string(),
                vec![],
                true,
            )
            .await
        }
    }
}

async fn ping(api: &dyn DiscordExecutor, interaction: &DiscordInteraction) -> Result<()> {
    api.reply(
        interaction.id,
        interaction.token.clone(),
        "PONG!".to_string(),
        vec![],
        false,
    )
    .await
}

// The buttons carry who asked ("choose:yes:<user id>"), so a click can be checked
// against the owner without storing anything.
async fn choose(api: &dyn DiscordExecutor, interaction: &DiscordInteraction) -> Result<()> {
    let buttons = vec![
        DiscordButton {
            label: "Yes".to_string(),
            custom_id: format!("choose:yes:{}", interaction.user_id),
        },
        DiscordButton {
            label: "No".to_string(),
            custom_id: format!("choose:no:{}", interaction.user_id),
        },
    ];
    let text = match interaction.option("question") {
        Some(question) => format!("Pick one: {}", question),
        None => "Pick one:".to_string(),
    };
    api.reply(
        interaction.id,
        interaction.token.clone(),
        text,
        buttons,
        false,
    )
    .await
}

// EX0036: longer than Discord's 3 seconds, so it defers first, then follows up.
async fn slow(api: &dyn DiscordExecutor, interaction: &DiscordInteraction) -> Result<()> {
    api.defer(
        interaction.id,
        interaction.token.clone(),
        Defer::Reply { private: false },
    )
    .await?;
    tokio::time::sleep(SLOW_DEMO_DELAY).await;
    api.follow_up(
        interaction.token.clone(),
        format!("Done after {} s", SLOW_DEMO_DELAY.as_secs()),
        vec![],
        false,
    )
    .await
}

async fn handle_button(
    api: &dyn DiscordExecutor,
    interaction: &DiscordInteraction,
    custom_id: &str,
) -> Result<()> {
    let Some((choice, owner_id)) = parse_choice(custom_id) else {
        return Err(anyhow!("unknown button: {}", custom_id));
    };
    if owner_id != interaction.user_id {
        // Only the person who clicked sees this; the message stays as it was.
        return api
            .reply(
                interaction.id,
                interaction.token.clone(),
                "This choice isn't yours.".to_string(),
                vec![],
                true,
            )
            .await;
    }
    api.update_message(
        interaction.id,
        interaction.token.clone(),
        format!("You chose: {}", choice),
        vec![],
    )
    .await
}

// "choose:yes:42" -> Some(("Yes", 42)). Anything else -> None.
pub fn parse_choice(custom_id: &str) -> Option<(&'static str, u64)> {
    let mut parts = custom_id.split(':');
    if parts.next()? != "choose" {
        return None;
    }
    let choice = match parts.next()? {
        "yes" => "Yes",
        "no" => "No",
        _ => return None,
    };
    let owner_id = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((choice, owner_id))
}
