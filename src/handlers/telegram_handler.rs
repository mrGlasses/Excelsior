// The Telegram bot: a background worker that long-polls Telegram and answers commands.
// Adding a command = one function below + one line in `handle_command`.
use crate::domain::telegram::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Update};
use crate::engine::telegram_engine::TelegramExecutor;
use anyhow::Result;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

// How long one getUpdates call waits for something to happen (Telegram allows up to 50).
pub const POLL_TIMEOUT_SECS: u64 = 25;
// After a network/API error, wait this long before polling again.
pub const RETRY_DELAY: Duration = Duration::from_secs(5);

// The worker. Fails (and so stops the app) only if the bot can't log in at all,
// e.g. a wrong token; network hiccups while polling are logged and retried.
pub async fn run_telegram_bot(
    api: Box<dyn TelegramExecutor>,
    shutdown: CancellationToken,
) -> Result<()> {
    let me = api.get_me().await?;
    info!(
        "telegram polling as @{}",
        me.username.unwrap_or_else(|| me.id.to_string())
    );

    // The id of the next update we want: everything below it is handled.
    let mut offset: Option<i64> = None;
    loop {
        let polled = tokio::select! {
            // `biased` = check the branches in order: once shutdown is requested, never
            // start another poll (by default select! picks a ready branch at random).
            biased;
            _ = shutdown.cancelled() => break,
            polled = api.get_updates(offset, POLL_TIMEOUT_SECS) => polled,
        };

        match polled {
            Ok(updates) => {
                for update in updates {
                    let update_id = update.update_id;
                    // A failing update is logged and skipped: one bad message must not
                    // block every message behind it.
                    if let Err(err) = handle_update(api.as_ref(), update).await {
                        warn!("telegram update {} failed: {}", update_id, err);
                    }
                    offset = Some(update_id + 1);
                }
            }
            Err(err) => {
                warn!(
                    "telegram polling failed, retrying in {}s: {}",
                    RETRY_DELAY.as_secs(),
                    err
                );
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    _ = tokio::time::sleep(RETRY_DELAY) => {}
                }
            }
        }
    }

    // Confirm what we've handled before leaving, so a restart doesn't answer it again.
    // timeout 0 = don't wait; an update this call returns is NOT confirmed, so it
    // simply comes back after the restart.
    if let Some(offset) = offset
        && let Err(err) = api.get_updates(Some(offset), 0).await
    {
        warn!("telegram could not confirm handled updates: {}", err);
    }
    Ok(())
}

pub async fn handle_update(api: &dyn TelegramExecutor, update: Update) -> Result<()> {
    if let Some(message) = update.message {
        // Only commands ("/something"); plain chat text is ignored.
        if let Some(text) = message.text
            && text.starts_with('/')
        {
            return handle_command(api, message.chat.id, &text).await;
        }
        return Ok(());
    }
    if let Some(query) = update.callback_query {
        return handle_button_tap(api, query).await;
    }
    Ok(())
}

async fn handle_command(api: &dyn TelegramExecutor, chat_id: i64, text: &str) -> Result<()> {
    match command_name(text) {
        "/ping" => ping(api, chat_id).await,
        "/choose" => choose(api, chat_id).await,
        _ => {
            api.send_message(chat_id, "Unknown command. Try /ping.".to_string(), None)
                .await
        }
    }
}

// "/ping" -> "/ping", "/ping@CardBot extra words" -> "/ping".
// In groups, Telegram adds "@botname" so several bots can share a command name.
pub fn command_name(text: &str) -> &str {
    let first_word = text.split_whitespace().next().unwrap_or("");
    first_word.split('@').next().unwrap_or("")
}

async fn ping(api: &dyn TelegramExecutor, chat_id: i64) -> Result<()> {
    api.send_message(chat_id, "PONG!".to_string(), None).await
}

async fn choose(api: &dyn TelegramExecutor, chat_id: i64) -> Result<()> {
    let keyboard = InlineKeyboardMarkup {
        inline_keyboard: vec![vec![
            InlineKeyboardButton {
                text: "Yes".to_string(),
                callback_data: "choose:yes".to_string(),
            },
            InlineKeyboardButton {
                text: "No".to_string(),
                callback_data: "choose:no".to_string(),
            },
        ]],
    };
    api.send_message(chat_id, "Pick one:".to_string(), Some(keyboard))
        .await
}

async fn handle_button_tap(api: &dyn TelegramExecutor, query: CallbackQuery) -> Result<()> {
    let choice = match query.data.as_deref() {
        Some("choose:yes") => Some("Yes"),
        Some("choose:no") => Some("No"),
        _ => None,
    };

    let edited = match (choice, &query.message) {
        (Some(choice), Some(message)) => {
            api.edit_message_text(
                message.chat.id,
                message.message_id,
                format!("You chose: {}", choice),
            )
            .await
        }
        _ => Ok(()),
    };

    // Always answer the tap, even if the edit failed, or the spinner keeps spinning.
    api.answer_callback_query(query.id).await?;
    edited
}
