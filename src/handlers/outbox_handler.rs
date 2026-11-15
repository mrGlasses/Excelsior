// The outbox: HTTP demo route, the worker that handles events, and the demo event handler.
// To use it for real, replace `handle_outbox_event` and write events with
// `insert_outbox_event(&mut tx, ...)` inside your own transactions.
use crate::domain::outbox::{DemoEventRequest, OutboxEvent};
use crate::engine::db_engine::create_demo_event_db_call;
use crate::engine::outbox_engine::{claim_next_event, mark_failed, mark_published};
use crate::state::AppState;
use anyhow::{Result, anyhow};
use axum::response::{IntoResponse, Response};
use axum::{Json, extract::State, http::StatusCode};
use sqlx::postgres::PgListener;
use sqlx::{Pool, Postgres};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

// The channel the outbox trigger notifies (see migration 0004_outbox.sql).
const OUTBOX_CHANNEL: &str = "outbox";
// Used when OUTBOX_MAX_ATTEMPTS is not set.
pub const DEFAULT_MAX_ATTEMPTS: i32 = 10;
// Even with no notification, look at the table this often (catches anything missed
// while the listener reconnects, and retries failing events).
pub const DEFAULT_RECHECK_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq)]
pub struct OutboxSettings {
    pub max_attempts: i32,
    pub recheck_interval: Duration,
}

// Settings from the OUTBOX_MAX_ATTEMPTS value; unset means the default.
pub fn outbox_settings_from(max_attempts: Option<String>) -> OutboxSettings {
    let max_attempts = match max_attempts {
        None => DEFAULT_MAX_ATTEMPTS,
        Some(text) => match text.trim().parse::<i32>() {
            Ok(number) if number > 0 => number,
            _ => panic!("OUTBOX_MAX_ATTEMPTS must be a number greater than 0."),
        },
    };
    OutboxSettings {
        max_attempts,
        recheck_interval: DEFAULT_RECHECK_INTERVAL,
    }
}

// POST /demo-events {"msg": "hi"} -> 201, and an event row in the outbox.
// {"msg": "rollback"} fails after the insert on purpose -> 500, and no event row.
pub async fn create_demo_event(
    State(state): State<AppState>,
    Json(payload): Json<DemoEventRequest>,
) -> Response {
    info!("create_demo_event called with params: {:?}", payload);
    match create_demo_event_db_call(State(state), payload.msg).await {
        Ok(_) => (StatusCode::CREATED, "CREATED").into_response(),
        Err(err) => {
            error!("Error occurred, demo event rolled back: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create demo event: {}", err),
            )
                .into_response()
        }
    }
}

// The demo event handler. Real apps replace this: send a message, call a service...
// It always fails on "boom", to show retries and giving up (OB-4).
pub async fn handle_outbox_event(event: &OutboxEvent) -> Result<()> {
    let msg = event.payload["msg"].as_str().unwrap_or("");
    if msg == "boom" {
        return Err(anyhow!("demo failure"));
    }
    info!("handled {}: {}", event.id, msg);
    Ok(())
}

// The worker. Sleeps until Postgres says an event committed (LISTEN/NOTIFY) or the
// recheck interval passes, then handles every pending event.
pub async fn run_outbox_worker(
    pool: Pool<Postgres>,
    settings: OutboxSettings,
    shutdown: CancellationToken,
) -> Result<()> {
    let mut listener = PgListener::connect_with(&pool).await?;
    listener.listen(OUTBOX_CHANNEL).await?;

    loop {
        // First, everything already waiting: events that committed while the app was
        // down (OB-3), or that a notification didn't cover.
        handle_pending_events(&pool, &settings, &shutdown).await?;

        tokio::select! {
            biased;
            _ = shutdown.cancelled() => return Ok(()),
            notification = listener.recv() => {
                if let Err(err) = notification {
                    // The listener reconnects by itself; the recheck covers the gap.
                    warn!("outbox listener error, rechecking soon: {}", err);
                    tokio::time::sleep(settings.recheck_interval).await;
                }
            }
            _ = tokio::time::sleep(settings.recheck_interval) => {}
        }
    }
}

// One pass over the pending events, oldest first. Each event is tried at most once per
// pass (`after_id` moves forward), so a failing event is retried on the next pass,
// not in a tight loop, and it never blocks the events behind it (OB-4).
pub async fn handle_pending_events(
    pool: &Pool<Postgres>,
    settings: &OutboxSettings,
    shutdown: &CancellationToken,
) -> Result<()> {
    let mut after_id = 0;
    while !shutdown.is_cancelled() {
        // One transaction per event: the row stays locked (SKIP LOCKED for everyone
        // else, OB-5) until it's marked and committed.
        let mut tx = pool.begin().await?;
        let Some(event) = claim_next_event(&mut tx, after_id).await? else {
            tx.commit().await?;
            return Ok(());
        };
        after_id = event.id;

        match handle_outbox_event(&event).await {
            Ok(()) => mark_published(&mut tx, event.id).await?,
            Err(err) => {
                let message = err.to_string();
                warn!(
                    "event {} failed (attempt {}): {}",
                    event.id, event.attempts, message
                );
                if mark_failed(&mut tx, event.id, &message, settings.max_attempts).await? {
                    error!(
                        "event {} gave up after {} attempts",
                        event.id, event.attempts
                    );
                }
            }
        }
        // A crash before this line rolls everything back, attempt included: the event
        // is handled again after the restart. Possibly twice, never zero times (OB-3).
        tx.commit().await?;
    }
    Ok(())
}
