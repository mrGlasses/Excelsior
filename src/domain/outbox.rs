use serde::{Deserialize, Serialize};

// One row of the outbox table, as the worker sees it after claiming it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OutboxEvent {
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
