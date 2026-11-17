// The http-external-call pack: the JSON the external service answers with.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Message {
    pub code: i32,
    pub message_text: String,
}
