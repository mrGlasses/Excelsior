// Talks to Telegram's Bot API: plain HTTPS + JSON, one POST per method
// (https://core.telegram.org/bots/api#making-requests).
use crate::domain::telegram::{
    BotUser, InlineKeyboardMarkup, PhotoSource, SentMessage, TelegramError, TelegramResponse,
    Update,
};
use anyhow::{Result, anyhow};
#[cfg(test)]
use mockall::automock;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_TELEGRAM_API_URL: &str = "https://api.telegram.org";
// EX0037: updates asked for per getUpdates call when TELEGRAM_UPDATE_LIMIT is not set
// (Telegram's maximum).
pub const DEFAULT_UPDATE_LIMIT: u32 = 100;

// Handlers use this trait instead of TelegramApi directly, so tests can pass a mock.
// AppState.telegram holds the real one, for sending from anywhere in the app.
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
    // Replaces a message's text. EX0040: and its buttons: `keyboard` None removes them.
    async fn edit_message_text(
        &self,
        chat_id: i64,
        message_id: i64,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()>;
    // Tells Telegram a button tap was handled, so the button's loading spinner stops.
    async fn answer_callback_query(&self, callback_query_id: String) -> Result<()>;
    // EX0037: sends a picture; returns the file_id Telegram gave it (the largest size),
    // to send it again later with PhotoSource::FileId, without uploading.
    async fn send_photo(
        &self,
        chat_id: i64,
        photo: PhotoSource,
        caption: Option<String>,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<String>;
}

// EX0037: what an app plugs into the Telegram bot (utils/pack_telegram_app.rs picks one).
// Updates from different chats run at the same time; from one chat, one after another.
#[async_trait::async_trait]
pub trait TelegramApp: Send + Sync {
    async fn handle(&self, api: Arc<dyn TelegramExecutor>, update: Update) -> Result<()>;
}

pub struct TelegramApi {
    client: reqwest::Client,
    // "<api url>/bot<token>": every method is "<base_url>/<method>".
    base_url: String,
    // EX0037: how many updates one getUpdates call may return.
    update_limit: u32,
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
            update_limit: DEFAULT_UPDATE_LIMIT,
        })
    }

    pub fn with_update_limit(mut self, update_limit: u32) -> Self {
        self.update_limit = update_limit;
        self
    }

    // EX0037: for AppState. None without TELEGRAM_BOT_TOKEN (the bot worker then stops the
    // app anyway). TELEGRAM_API_URL points it at a fake server in tests.
    pub fn from_env(max_poll_secs: u64) -> Option<Arc<TelegramApi>> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        if token.trim().is_empty() {
            return None;
        }
        let api_url = std::env::var("TELEGRAM_API_URL")
            .unwrap_or_else(|_| DEFAULT_TELEGRAM_API_URL.to_string());
        let limit = update_limit_from(std::env::var("TELEGRAM_UPDATE_LIMIT").ok());
        let api = TelegramApi::new(&api_url, &token, max_poll_secs)
            .expect("Failed to build the Telegram HTTP client")
            .with_update_limit(limit);
        Some(Arc::new(api))
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, body: Value) -> Result<T> {
        let request = self
            .client
            .post(format!("{}/{}", self.base_url, method))
            .json(&body);
        self.send(method, request).await
    }

    async fn send<T: DeserializeOwned>(
        &self,
        method: &str,
        request: reqwest::RequestBuilder,
    ) -> Result<T> {
        let response: TelegramResponse<T> = request
            .send()
            .await
            // The URL contains the bot token: never let it reach the logs.
            .map_err(|err| err.without_url())?
            .json()
            .await
            .map_err(|err| err.without_url())?;
        into_result(method, response)
    }

    // A photo upload as multipart/form-data, written by hand (reqwest's `multipart`
    // feature would change the shared reqwest line in Cargo.toml).
    async fn upload_photo(
        &self,
        chat_id: i64,
        name: &str,
        data: &[u8],
        caption: Option<String>,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<SentMessage> {
        let mut fields = vec![("chat_id".to_string(), chat_id.to_string())];
        if let Some(caption) = caption {
            fields.push(("caption".to_string(), caption));
        }
        if let Some(keyboard) = keyboard {
            fields.push((
                "reply_markup".to_string(),
                serde_json::to_string(&keyboard)?,
            ));
        }
        let (content_type, body) = multipart_body(&fields, "photo", name, data);
        let request = self
            .client
            .post(format!("{}/sendPhoto", self.base_url))
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body);
        self.send("sendPhoto", request).await
    }
}

// EX0037: an error answer -> TelegramError (inside anyhow), so callers can tell a rate
// limit (retry later) from a blocked user (give up).
pub fn into_result<T>(method: &str, response: TelegramResponse<T>) -> Result<T> {
    if let (true, Some(result)) = (response.ok, response.result) {
        return Ok(result);
    }
    let method = method.to_string();
    let description = response
        .description
        .unwrap_or_else(|| "no description".to_string());
    let retry_after = response.parameters.and_then(|p| p.retry_after);
    let error = match (response.error_code, retry_after) {
        (Some(429), retry_after) => TelegramError::RateLimited {
            method,
            retry_after: retry_after.unwrap_or(1),
        },
        (Some(403), _) => TelegramError::Forbidden {
            method,
            description,
        },
        (Some(400), _) => TelegramError::BadRequest {
            method,
            description,
        },
        _ => TelegramError::Other {
            method,
            description,
        },
    };
    Err(anyhow!(error))
}

// TELEGRAM_UPDATE_LIMIT: unset = 100; otherwise 1 to 100 (Telegram's range).
pub fn update_limit_from(value: Option<String>) -> u32 {
    match value {
        None => DEFAULT_UPDATE_LIMIT,
        Some(text) => match text.trim().parse::<u32>() {
            Ok(limit) if (1..=100).contains(&limit) => limit,
            _ => panic!("TELEGRAM_UPDATE_LIMIT must be a number from 1 to 100."),
        },
    }
}

// Text fields plus one file, as multipart/form-data. Returns (Content-Type, body).
pub fn multipart_body(
    fields: &[(String, String)],
    file_field: &str,
    file_name: &str,
    data: &[u8],
) -> (String, Vec<u8>) {
    // Random enough to never appear inside an image.
    let boundary = format!("excelsior-{:032x}", rand_u128());
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    // Quotes would end the file name early.
    let safe_name = file_name.replace(['"', '\r', '\n'], "_");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{file_field}\"; filename=\"{safe_name}\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

// A random number from the standard library only (RandomState is seeded randomly).
fn rand_u128() -> u128 {
    use std::hash::{BuildHasher, Hasher};
    let state = std::collections::hash_map::RandomState::new();
    let mut high = state.build_hasher();
    high.write_u8(1);
    let mut low = state.build_hasher();
    low.write_u8(2);
    ((high.finish() as u128) << 64) | low.finish() as u128
}

// The file_id of the largest size Telegram stored.
fn largest_file_id(message: SentMessage) -> Result<String> {
    message
        .photo
        .into_iter()
        .max_by_key(|size| size.width * size.height)
        .map(|size| size.file_id)
        .ok_or_else(|| anyhow!("telegram sendPhoto: the answer has no photo sizes"))
}

#[async_trait::async_trait]
impl TelegramExecutor for TelegramApi {
    async fn get_me(&self) -> Result<BotUser> {
        self.call("getMe", json!({})).await
    }

    async fn get_updates(&self, offset: Option<i64>, timeout_secs: u64) -> Result<Vec<Update>> {
        // EX0037: up to `update_limit` per call. The bot confirms them (the next call's
        // offset) only after the whole batch is handled, so a restart repeats a batch
        // rather than losing it.
        self.call(
            "getUpdates",
            json!({
                "offset": offset,
                "timeout": timeout_secs,
                "limit": self.update_limit,
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

    async fn edit_message_text(
        &self,
        chat_id: i64,
        message_id: i64,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<()> {
        let mut body = json!({ "chat_id": chat_id, "message_id": message_id, "text": text });
        if let Some(keyboard) = keyboard {
            body["reply_markup"] = serde_json::to_value(keyboard)?;
        }
        // The result is the edited message (or `true`); we don't need it.
        let _: Value = self.call("editMessageText", body).await?;
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

    async fn send_photo(
        &self,
        chat_id: i64,
        photo: PhotoSource,
        caption: Option<String>,
        keyboard: Option<InlineKeyboardMarkup>,
    ) -> Result<String> {
        let sent: SentMessage = match photo {
            PhotoSource::FileId(text) | PhotoSource::Url(text) => {
                let mut body = json!({ "chat_id": chat_id, "photo": text });
                if let Some(caption) = caption {
                    body["caption"] = json!(caption);
                }
                if let Some(keyboard) = keyboard {
                    body["reply_markup"] = serde_json::to_value(keyboard)?;
                }
                self.call("sendPhoto", body).await?
            }
            PhotoSource::Bytes { name, data } => {
                self.upload_photo(chat_id, &name, &data, caption, keyboard)
                    .await?
            }
        };
        largest_file_id(sent)
    }
}
