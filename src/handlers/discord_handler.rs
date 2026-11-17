// The Discord bot: a background worker holding the gateway connection (twilight-gateway)
// and answering slash commands and button clicks.
// Adding a command = one line in `commands()` + one line in `handle_command`.
use crate::domain::discord::{DiscordButton, DiscordCommand, DiscordInteraction, InteractionKind};
use crate::engine::discord_engine::{DiscordApi, DiscordExecutor, to_discord_interaction};
use crate::state::AppState;
use crate::utils::workers::Workers;
use anyhow::{Result, anyhow};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use twilight_gateway::{
    CloseFrame, ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_model::user::CurrentUser;

// After asking the gateway to close, wait at most this long for it to finish.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(3);

pub struct DiscordSettings {
    pub token: String,
    // Register commands in this server only (instant, for development); None = everywhere.
    pub guild_id: Option<u64>,
    // Only for tests: connect to this address instead of Discord's gateway.
    pub gateway_url: Option<String>,
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

// The slash commands the bot offers.
pub fn commands() -> Vec<DiscordCommand> {
    vec![
        DiscordCommand {
            name: "ping".to_string(),
            description: "Replies PONG!".to_string(),
        },
        DiscordCommand {
            name: "choose".to_string(),
            description: "Pick Yes or No with buttons".to_string(),
        },
    ]
}

// Called from utils/pack_workers.rs. False (the app exits with code 1) without a token.
pub fn start(workers: &mut Workers, _state: &AppState) -> bool {
    let token = match std::env::var("DISCORD_TOKEN") {
        Ok(token) if !token.trim().is_empty() => token,
        _ => {
            error!("DISCORD_TOKEN is missing");
            return false;
        }
    };
    let settings = DiscordSettings {
        token,
        guild_id: discord_guild_id_from(std::env::var("DISCORD_GUILD_ID").ok()),
        gateway_url: std::env::var("DISCORD_GATEWAY_URL").ok(),
    };
    workers.spawn("discord", move |shutdown| {
        run_discord_bot(settings, shutdown)
    });
    true
}

// The worker. Fails (and so stops the app) only when Discord closes the connection for
// good, e.g. a wrong token; network drops are reconnected by twilight by itself.
pub async fn run_discord_bot(settings: DiscordSettings, shutdown: CancellationToken) -> Result<()> {
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
    let mut api: Option<DiscordApi> = None;

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
                let discord = DiscordApi::new(settings.token.clone(), ready.application.id);
                let registered = discord
                    .register_commands(settings.guild_id, commands())
                    .await?;
                match settings.guild_id {
                    Some(guild_id) => {
                        info!("registered {} commands in guild {}", registered, guild_id)
                    }
                    None => info!("registered {} commands globally", registered),
                }
                api = Some(discord);
            }
            Some(Ok(Event::InteractionCreate(event))) => {
                let (Some(api), Some(interaction)) = (&api, to_discord_interaction(&event.0))
                else {
                    continue;
                };
                // A failing answer is logged and skipped: one bad click must not stop the bot.
                if let Err(err) = handle_interaction(api, interaction).await {
                    warn!("discord interaction failed: {}", err);
                }
            }
            Some(Ok(_)) => {}
        }
    }

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
    api.reply(
        interaction.id,
        interaction.token.clone(),
        "Pick one:".to_string(),
        buttons,
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
