// The Telegram bot: a background worker that long-polls Telegram and hands each update to
// the app (utils/pack_telegram_app.rs picks it; DemoTelegramApp below by default).
// EX0037: updates from different chats are handled at the same time; updates from the same
// chat stay in order.
use crate::domain::telegram::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Update};
use crate::engine::telegram_engine::{TelegramApi, TelegramApp, TelegramExecutor};
use crate::state::AppState;
use crate::utils::workers::Workers;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

// How long one getUpdates call waits for something to happen (Telegram allows up to 50).
pub const POLL_TIMEOUT_SECS: u64 = 25;
// After a network/API error, wait this long before polling again.
pub const RETRY_DELAY: Duration = Duration::from_secs(5);
// EX0037: one update may take at most this long; then it's dropped (and logged).
pub const UPDATE_TIMEOUT: Duration = Duration::from_secs(30);
// EX0037: on shutdown, the batch being handled gets this long to finish.
pub const IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(3);
// Used when TELEGRAM_MAX_CONCURRENT is not set: chats handled at the same time.
pub const DEFAULT_MAX_CONCURRENT: usize = 16;

// TELEGRAM_MAX_CONCURRENT: unset = the default; otherwise a number greater than 0.
pub fn max_concurrent_from(value: Option<String>) -> usize {
    match value {
        None => DEFAULT_MAX_CONCURRENT,
        Some(text) => match text.trim().parse::<usize>() {
            Ok(number) if number > 0 => number,
            _ => panic!("TELEGRAM_MAX_CONCURRENT must be a number greater than 0."),
        },
    }
}

// Called from utils/pack_workers.rs. False (the app exits with code 1) without a token.
pub fn start(workers: &mut Workers, state: &AppState) -> bool {
    match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(token) if !token.trim().is_empty() => {}
        _ => {
            error!("TELEGRAM_BOT_TOKEN is missing");
            return false;
        }
    };
    // The same client as AppState.telegram.
    let api: Arc<dyn TelegramExecutor> = match &state.telegram {
        Some(api) => api.clone(),
        None => TelegramApi::from_env(POLL_TIMEOUT_SECS).expect("the token was checked above"),
    };
    let app = crate::utils::pack_telegram_app::telegram_app(state);
    let max_concurrent = max_concurrent_from(std::env::var("TELEGRAM_MAX_CONCURRENT").ok());
    workers.spawn("telegram", move |shutdown| {
        run_telegram_bot(api, app, max_concurrent, shutdown)
    });
    true
}

// The worker. Fails (and so stops the app) only if the bot can't log in at all,
// e.g. a wrong token; network hiccups while polling are logged and retried.
pub async fn run_telegram_bot(
    api: Arc<dyn TelegramExecutor>,
    app: Arc<dyn TelegramApp>,
    max_concurrent: usize,
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
            Ok(updates) if updates.is_empty() => {}
            Ok(updates) => {
                let next = updates.iter().map(|u| u.update_id).max().map(|id| id + 1);
                // Only a fully handled batch is confirmed; an interrupted one comes back
                // after the restart (apps dedupe by update_id).
                if handle_batch(&api, &app, updates, max_concurrent, &shutdown).await {
                    offset = next;
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

// EX0037: the batch split by chat, in the order each chat first appears. Updates without a
// chat get a group of their own.
pub fn group_by_chat(updates: Vec<Update>) -> Vec<Vec<Update>> {
    let mut groups: Vec<(Option<i64>, Vec<Update>)> = Vec::new();
    for update in updates {
        let chat = update.chat_id();
        match groups
            .iter_mut()
            .find(|(id, _)| chat.is_some() && *id == chat)
        {
            Some((_, group)) => group.push(update),
            None => groups.push((chat, vec![update])),
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

// Handles one batch: chats in parallel (at most `max_concurrent`), each chat in order.
// True when every update was handled (successfully or not); false when shutdown cut it.
pub async fn handle_batch(
    api: &Arc<dyn TelegramExecutor>,
    app: &Arc<dyn TelegramApp>,
    updates: Vec<Update>,
    max_concurrent: usize,
    shutdown: &CancellationToken,
) -> bool {
    let permits = Arc::new(Semaphore::new(max_concurrent));
    let mut tasks = JoinSet::new();
    for group in group_by_chat(updates) {
        let (api, app, permits) = (api.clone(), app.clone(), permits.clone());
        tasks.spawn(async move {
            let _permit = permits.acquire_owned().await;
            for update in group {
                let update_id = update.update_id;
                // A failing or stuck update is logged and skipped: one bad message must
                // not block every message behind it.
                match tokio::time::timeout(UPDATE_TIMEOUT, app.handle(api.clone(), update)).await {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => warn!("telegram update {} failed: {}", update_id, err),
                    Err(_) => warn!(
                        "telegram update {} took more than {}s, dropped",
                        update_id,
                        UPDATE_TIMEOUT.as_secs()
                    ),
                }
            }
        });
    }

    // Wait for every chat. On shutdown, give them IN_FLIGHT_TIMEOUT more, then drop the rest.
    let mut deadline: Option<tokio::time::Instant> = None;
    loop {
        let give_up_at = deadline.unwrap_or_else(tokio::time::Instant::now);
        tokio::select! {
            biased;
            next = tasks.join_next() => {
                if next.is_none() {
                    return true;
                }
            }
            _ = shutdown.cancelled(), if deadline.is_none() => {
                deadline = Some(tokio::time::Instant::now() + IN_FLIGHT_TIMEOUT);
            }
            _ = tokio::time::sleep_until(give_up_at), if deadline.is_some() => {
                warn!("telegram shutdown: updates still running were dropped (they'll come back)");
                tasks.abort_all();
                return false;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// The demo app: what the bot does until an app plugs in its own (pack_telegram_app.rs).
// ---------------------------------------------------------------------------------------

pub struct DemoTelegramApp;

#[async_trait::async_trait]
impl TelegramApp for DemoTelegramApp {
    async fn handle(&self, api: Arc<dyn TelegramExecutor>, update: Update) -> Result<()> {
        handle_update(api.as_ref(), update).await
    }
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
