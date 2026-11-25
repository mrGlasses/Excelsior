// The parts of Telegram's Bot API JSON that the bot uses (https://core.telegram.org/bots/api).
// Telegram sends many more fields; serde ignores the ones not listed here.
use serde::{Deserialize, Serialize};

// Every Bot API answer has this envelope: {"ok": true, "result": ...}
// or {"ok": false, "error_code": 429, "description": "...", "parameters": {...}}.
#[derive(Deserialize, Debug)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
    // EX0037: the error's HTTP-like code (400, 403, 429...), and for 429 how long to wait.
    pub error_code: Option<i64>,
    pub parameters: Option<ResponseParameters>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ResponseParameters {
    pub retry_after: Option<u64>,
}

// EX0037: the Bot API errors an app needs to tell apart. They come wrapped in
// anyhow::Error: `err.downcast_ref::<TelegramError>()`.
#[derive(Debug, Clone, PartialEq)]
pub enum TelegramError {
    // 429: too many messages; try again after this many seconds.
    RateLimited { method: String, retry_after: u64 },
    // 403: the user blocked the bot (or never started it); retrying won't help.
    Forbidden { method: String, description: String },
    // 400: something in the request is wrong (unknown chat, bad file_id...).
    BadRequest { method: String, description: String },
    Other { method: String, description: String },
}

impl std::fmt::Display for TelegramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelegramError::RateLimited {
                method,
                retry_after,
            } => write!(
                f,
                "telegram {method}: too many requests, retry after {retry_after}s"
            ),
            TelegramError::Forbidden {
                method,
                description,
            } => write!(f, "telegram {method}: forbidden: {description}"),
            TelegramError::BadRequest {
                method,
                description,
            } => write!(f, "telegram {method}: bad request: {description}"),
            TelegramError::Other {
                method,
                description,
            } => write!(f, "telegram {method} failed: {description}"),
        }
    }
}

impl std::error::Error for TelegramError {}

// The bot itself, from getMe.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct BotUser {
    pub id: i64,
    pub username: Option<String>,
}

// EX0037: who sent a message or tapped a button.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct TelegramUser {
    pub id: i64,
    #[serde(default)]
    pub is_bot: bool,
    #[serde(default)]
    pub first_name: String,
    pub username: Option<String>,
    // The user's app language, e.g. "pt-br" or "en". Not always sent.
    pub language_code: Option<String>,
}

// One event from getUpdates: a new message, a button tap, or something we don't handle.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<IncomingMessage>,
    pub callback_query: Option<CallbackQuery>,
}

impl Update {
    // EX0037: the chat this update belongs to (updates from one chat are handled in order).
    pub fn chat_id(&self) -> Option<i64> {
        if let Some(message) = &self.message {
            return Some(message.chat.id);
        }
        let query = self.callback_query.as_ref()?;
        match (&query.message, &query.from) {
            (Some(message), _) => Some(message.chat.id),
            (None, Some(from)) => Some(from.id),
            (None, None) => None,
        }
    }

    // EX0037: who it came from.
    pub fn from(&self) -> Option<&TelegramUser> {
        match (&self.message, &self.callback_query) {
            (Some(message), _) => message.from.as_ref(),
            (None, Some(query)) => query.from.as_ref(),
            (None, None) => None,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct IncomingMessage {
    pub message_id: i64,
    // EX0037: missing only for messages sent on behalf of a channel.
    pub from: Option<TelegramUser>,
    pub chat: Chat,
    pub text: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Chat {
    pub id: i64,
    // EX0037: "private", "group", "supergroup" or "channel".
    #[serde(rename = "type", default)]
    pub kind: String,
}

// A tap on an inline button. `message` is the message the button belongs to.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct CallbackQuery {
    pub id: String,
    // EX0037: who tapped (Telegram always sends it).
    pub from: Option<TelegramUser>,
    pub data: Option<String>,
    pub message: Option<IncomingMessage>,
}

// Buttons shown under a message: a list of rows, each row a list of buttons.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

// `callback_data` comes back in CallbackQuery.data when the button is tapped.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct InlineKeyboardButton {
    pub text: String,
    pub callback_data: String,
}

// EX0037: the picture to send with send_photo.
#[derive(Debug, Clone, PartialEq)]
pub enum PhotoSource {
    // A file Telegram already has (the file_id a previous send_photo returned). Instant,
    // and only valid for the bot that uploaded it.
    FileId(String),
    // An image on the internet; Telegram downloads it.
    Url(String),
    // Upload the bytes (multipart). Returns a file_id to reuse afterwards.
    Bytes { name: String, data: Vec<u8> },
}

// The part of a sent message we read: its photo sizes (the last one is the largest).
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SentMessage {
    pub message_id: i64,
    #[serde(default)]
    pub photo: Vec<PhotoSize>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct PhotoSize {
    pub file_id: String,
    pub width: i64,
    pub height: i64,
}
