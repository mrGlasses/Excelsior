use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Message {
    pub code: i32,
    pub message_text: String,
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

pub struct ReplaceParams {
    pub old_str: String,
    pub new_str: String,
}
