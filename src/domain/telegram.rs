// The parts of Telegram's Bot API JSON that the bot uses (https://core.telegram.org/bots/api).
// Telegram sends many more fields; serde ignores the ones not listed here.
use serde::{Deserialize, Serialize};

// Every Bot API answer has this envelope: {"ok": true, "result": ...}
// or {"ok": false, "description": "..."}.
#[derive(Deserialize, Debug)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

// The bot itself, from getMe.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct BotUser {
    pub id: i64,
    pub username: Option<String>,
}

// One event from getUpdates: a new message, a button tap, or something we don't handle.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<IncomingMessage>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct IncomingMessage {
    pub message_id: i64,
    pub chat: Chat,
    pub text: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Chat {
    pub id: i64,
}

// A tap on an inline button. `message` is the message the button belongs to.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct CallbackQuery {
    pub id: String,
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
