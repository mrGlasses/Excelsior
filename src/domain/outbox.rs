use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// One row of the outbox table, as the worker sees it after claiming it.
#[derive(Serialize, Deserialize, FromRow, Debug, Clone, PartialEq)]
pub struct OutboxEvent {
    #[sqlx(rename = "event_id")]
    pub id: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    // Including the current one.
    pub attempts: i32,
}

// Body of POST /demo-events: {"msg": "hi"}
#[derive(Deserialize, Debug)]
pub struct DemoEventRequest {
    pub msg: String,
}
