use crate::domain::outbox::{DemoEventRequest, OutboxEvent};
use crate::engine::db_engine::{DbPool, MockDatabaseExecutor};
use crate::handlers::outbox_handler::*;
use crate::state::AppState;
use anyhow::anyhow;
use axum::http::StatusCode;
use axum::{Json, extract::State};
use mockall::predicate::*;
use std::sync::Arc;
use std::time::Duration;

fn state_with(mock_db: MockDatabaseExecutor) -> AppState {
    AppState {
        db_pool: Arc::new(DbPool::Mock(mock_db)),
        ..Default::default()
    }
}

fn event_with_msg(msg: &str) -> OutboxEvent {
    OutboxEvent {
        id: 7,
        event_type: "DemoEvent".to_string(),
        payload: serde_json::json!({ "msg": msg }),
        attempts: 1,
    }
}

#[tokio::test]
async fn test_create_demo_event_success() {
    let mut mock_db = MockDatabaseExecutor::new();
    mock_db
        .expect_execute_create_demo_event()
        .with(eq("hi".to_string()))
        .times(1)
        .returning(|_| Ok("OK".to_string()));

    let response = create_demo_event(
        State(state_with(mock_db)),
        Json(DemoEventRequest {
            msg: "hi".to_string(),
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_demo_event_rolled_back() {
    let mut mock_db = MockDatabaseExecutor::new();
    mock_db
        .expect_execute_create_demo_event()
        .times(1)
        .returning(|_| Err(anyhow!("demo failure after insert, rolled back")));

    let response = create_demo_event(
        State(state_with(mock_db)),
        Json(DemoEventRequest {
            msg: "rollback".to_string(),
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_demo_handler_accepts_normal_events() {
    assert!(handle_outbox_event(&event_with_msg("hi")).await.is_ok());
}

#[tokio::test]
async fn test_demo_handler_always_fails_on_boom() {
    let err = handle_outbox_event(&event_with_msg("boom"))
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "demo failure");
}

#[tokio::test]
async fn test_demo_handler_without_msg_is_ok() {
    let mut event = event_with_msg("x");
    event.payload = serde_json::json!({ "something": "else" });

    assert!(handle_outbox_event(&event).await.is_ok());
}

#[test]
fn test_outbox_settings_default() {
    assert_eq!(
        outbox_settings_from(None),
        OutboxSettings {
            max_attempts: 10,
            recheck_interval: Duration::from_secs(5),
            retry_base: Duration::from_secs(5),
        }
    );
}

#[test]
fn test_outbox_settings_from_env_value() {
    assert_eq!(
        outbox_settings_from(Some(" 3 ".to_string())).max_attempts,
        3
    );
}

#[test]
#[should_panic(expected = "OUTBOX_MAX_ATTEMPTS must be a number from 1 to 100.")]
fn test_outbox_settings_zero_panics() {
    outbox_settings_from(Some("0".to_string()));
}

#[test]
fn test_retry_base_default() {
    assert_eq!(retry_base_from(None), Duration::from_secs(5));
}

#[test]
fn test_retry_base_from_env_value() {
    assert_eq!(
        retry_base_from(Some(" 30 ".to_string())),
        Duration::from_secs(30)
    );
    // 0 = retry on the next pass.
    assert_eq!(retry_base_from(Some("0".to_string())), Duration::ZERO);
}

#[test]
#[should_panic(expected = "OUTBOX_RETRY_BASE_SECS must be a whole number of seconds (0 or more).")]
fn test_retry_base_rejects_text() {
    retry_base_from(Some("five".to_string()));
}

#[test]
#[should_panic(expected = "OUTBOX_MAX_ATTEMPTS must be a number from 1 to 100.")]
fn test_outbox_settings_above_100_panics() {
    outbox_settings_from(Some("101".to_string()));
}

#[test]
fn test_outbox_settings_accepts_100() {
    assert_eq!(
        outbox_settings_from(Some("100".to_string())).max_attempts,
        100
    );
}
