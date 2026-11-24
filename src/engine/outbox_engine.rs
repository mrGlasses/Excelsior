// Outbox database calls. They take a connection, so they work inside a transaction:
// `insert_outbox_event(&mut tx, ...)`.
use crate::domain::outbox::OutboxEvent;
use anyhow::Result;
use serde_json::Value;
use sqlx::{MySqlConnection, Row, query};
use std::sync::LazyLock;
use tokio::sync::Notify;

// MariaDB has no LISTEN/NOTIFY, so the app wakes its own worker: after committing an
// event, call `wake_outbox_worker()`. A wake-up sent while the worker is busy isn't
// lost: Notify keeps it until the worker waits again. Inserts from outside the app
// (another copy, a SQL client) are caught by the worker's 1-second re-check.
static OUTBOX_WAKEUP: LazyLock<Notify> = LazyLock::new(Notify::new);

pub fn wake_outbox_worker() {
    OUTBOX_WAKEUP.notify_one();
}

pub async fn outbox_wakeup() {
    OUTBOX_WAKEUP.notified().await;
}

// Writes an event. Use it in the same transaction as the change it describes:
// if that transaction rolls back, the event disappears with it.
pub async fn insert_outbox_event(
    conn: &mut MySqlConnection,
    event_type: &str,
    payload: &Value,
) -> Result<()> {
    query("CALL sp_insert_outbox_event(?, ?)")
        .bind(event_type)
        .bind(payload.to_string())
        .execute(conn)
        .await?;
    Ok(())
}

// Locks and returns the next pending event after `after_id`, or None when there's
// nothing left. The lock lasts until the transaction ends.
pub async fn claim_next_event(
    conn: &mut MySqlConnection,
    after_id: i64,
) -> Result<Option<OutboxEvent>> {
    let row = query("CALL sp_claim_outbox_event(?)")
        .bind(after_id)
        .fetch_optional(conn)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    // MariaDB keeps JSON as text: read it as a string, then parse it.
    let payload: String = row.try_get("payload")?;
    Ok(Some(OutboxEvent {
        id: row.try_get("event_id")?,
        event_type: row.try_get("event_type")?,
        payload: serde_json::from_str(&payload)?,
        attempts: row.try_get("attempts")?,
    }))
}

pub async fn mark_published(conn: &mut MySqlConnection, event_id: i64) -> Result<()> {
    query("CALL sp_mark_outbox_published(?)")
        .bind(event_id)
        .execute(conn)
        .await?;
    Ok(())
}

// Returns true when the event used its last attempt and was set aside. Otherwise the
// event waits retry_base * 2^(attempts - 1) (at most an hour) before its next try.
pub async fn mark_failed(
    conn: &mut MySqlConnection,
    event_id: i64,
    error: &str,
    max_attempts: i32,
    retry_base: std::time::Duration,
) -> Result<bool> {
    let retry_base_secs = i32::try_from(retry_base.as_secs()).unwrap_or(i32::MAX);
    let row = query("CALL sp_mark_outbox_failed(?, ?, ?, ?)")
        .bind(event_id)
        .bind(error)
        .bind(max_attempts)
        .bind(retry_base_secs)
        .fetch_one(conn)
        .await?;
    let gave_up: i64 = row.try_get("gave_up")?;
    Ok(gave_up != 0)
}
