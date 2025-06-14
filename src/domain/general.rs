use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub code: i32,
    pub message_text: String,
}

impl Default for Message {
    fn default() -> Message {
        Message {
            code: Default::default(),
            message_text: Default::default(),
        }
    }
}

#[derive(Deserialize)]
pub struct Params {
    pub param_1: u32,
    pub param_2: String,
}

#[derive(Deserialize)]
pub struct FilterParams {
    pub name: Option<String>,
    pub age: Option<u32>,
    pub active: Option<bool>,
}
