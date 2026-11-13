// Talks to Telegram's Bot API: plain HTTPS + JSON, one POST per method
// (https://core.telegram.org/bots/api#making-requests).
use crate::domain::telegram::{BotUser, InlineKeyboardMarkup, TelegramResponse, Update};
use anyhow::{Result, anyhow};
#[cfg(test)]
use mockall::automock;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::time::Duration;

pub const DEFAULT_TELEGRAM_API_URL: &str = "https://api.telegram.org";

// Handlers use this trait instead of TelegramApi directly, so tests can pass a mock.
#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait TelegramExecutor: Send + Sync {
    async fn get_me(&self) -> Result<BotUser>;
    // Waits up to `timeout_secs` for new updates (long polling). Calling it with an
    // `offset` confirms every update below that id: Telegram won't send them again.
    async fn get_updates(&self, offset: Option<i64>, timeout_secs: u64) -> Result<Vec<Update>>;
    async fn send_message(
        &self,
        chat_id: i64,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;
    // Replaces a message's text. Its inline buttons disappear, since none are sent.
    async fn edit_message_text(&self, chat_id: i64, message_id: i64, text: String) -> Result<()>;
    // Tells Telegram a button tap was handled, so the button's loading spinner stops.
    async fn answer_callback_query(&self, callback_query_id: String) -> Result<()>;
}

pub struct TelegramApi {
    client: reqwest::Client,
    // "<api url>/bot<token>": every method is "<base_url>/<method>".
    base_url: String,
}

impl TelegramApi {
    // `api_url` is https://api.telegram.org in production; tests point it at a fake server.
    // `max_poll_secs` is the longest getUpdates wait we'll ask for: the HTTP timeout
    // must be longer than that, or every quiet poll would end in a timeout error.
    pub fn new(api_url: &str, token: &str, max_poll_secs: u64) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(max_poll_secs + 10))
            .build()?;
        Ok(TelegramApi {
            client,
            base_url: format!("{}/bot{}", api_url.trim_end_matches('/'), token),
        })
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, body: Value) -> Result<T> {
        let response: TelegramResponse<T> = self
            .client
            .post(format!("{}/{}", self.base_url, method))
            .json(&body)
            .send()
            .await
            // The URL contains the bot token: never let it reach the logs.
            .map_err(|err| err.without_url())?
            .json()
            .await
            .map_err(|err| err.without_url())?;

        match (response.ok, response.result) {
            (true, Some(result)) => Ok(result),
            _ => Err(anyhow!(
                "telegram {} failed: {}",
                method,
                response
                    .description
                    .unwrap_or_else(|| "no description".to_string())
            )),
        }
    }
}

#[async_trait::async_trait]
impl TelegramExecutor for TelegramApi {
    async fn get_me(&self) -> Result<BotUser> {
        self.call("getMe", json!({})).await
    }

    async fn get_updates(&self, offset: Option<i64>, timeout_secs: u64) -> Result<Vec<Update>> {
        // One update per call: the next call's offset confirms it right after it's
        // handled, so a restart neither drops nor repeats messages.
        self.call(
            "getUpdates",
            json!({
                "offset": offset,
                "timeout": timeout_secs,
                "limit": 1,
                "allowed_updates": ["message", "callback_query"],
            }),
        )
        .await
    }

    async fn send_message(
        &self,
        chat_id: i64,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()> {
        let mut body = json!({ "chat_id": chat_id, "text": text });
        if let Some(keyboard) = keyboard {
            body["reply_markup"] = serde_json::to_value(keyboard)?;
        }
        // The result is the sent message; we don't need it.
        let _: Value = self.call("sendMessage", body).await?;
        Ok(())
    }

    async fn edit_message_text(&self, chat_id: i64, message_id: i64, text: String) -> Result<()> {
        let _: Value = self
            .call(
                "editMessageText",
                json!({ "chat_id": chat_id, "message_id": message_id, "text": text }),
            )
            .await?;
        Ok(())
    }

    async fn answer_callback_query(&self, callback_query_id: String) -> Result<()> {
        let _: Value = self
            .call(
                "answerCallbackQuery",
                json!({ "callback_query_id": callback_query_id }),
            )
            .await?;
        Ok(())
    }
}
