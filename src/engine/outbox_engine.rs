// Outbox database calls. Like `insert_user`, they take a connection, so they work
// inside a transaction: `insert_outbox_event(&mut tx, ...)`.
use crate::domain::outbox::OutboxEvent;
use anyhow::Result;
use serde_json::Value;
use sqlx::{PgConnection, query, query_as, query_scalar};

// Writes an event. Use it in the same transaction as the change it describes:
// if that transaction rolls back, the event disappears with it.
pub async fn insert_outbox_event(
    conn: &mut PgConnection,
    event_type: &str,
    payload: &Value,
) -> Result<()> {
    query("CALL sp_insert_outbox_event($1, $2)")
        .bind(event_type)
        .bind(payload)
        .execute(conn)
        .await?;
    Ok(())
}

// Locks and returns the next pending event after `after_id`, or None when there's
// nothing left. The lock lasts until the transaction ends.
pub async fn claim_next_event(
    conn: &mut PgConnection,
    after_id: i64,
) -> Result<Option<OutboxEvent>> {
    let event = query_as::<_, OutboxEvent>("SELECT * FROM sp_claim_outbox_event($1)")
        .bind(after_id)
        .fetch_optional(conn)
        .await?;
    Ok(event)
}

pub async fn mark_published(conn: &mut PgConnection, event_id: i64) -> Result<()> {
    query("CALL sp_mark_outbox_published($1)")
        .bind(event_id)
        .execute(conn)
        .await?;
    Ok(())
}

// Returns true when the event used its last attempt and was set aside.
pub async fn mark_failed(
    conn: &mut PgConnection,
    event_id: i64,
    error: &str,
    max_attempts: i32,
) -> Result<bool> {
    let gave_up: bool = query_scalar("SELECT sp_mark_outbox_failed($1, $2, $3)")
        .bind(event_id)
        .bind(error)
        .bind(max_attempts)
        .fetch_one(conn)
        .await?;
    Ok(gave_up)
}
