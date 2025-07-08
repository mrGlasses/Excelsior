use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, Deserialize, FromRow, Debug, Default)]
pub struct Message {
    pub code: i32,
    pub message_text: String,
}
