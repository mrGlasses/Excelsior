use crate::domain::cache::CacheValue;
use crate::engine::cache_engine::{CachePool, MockCacheExecutor};
use crate::handlers::cache_handler::*;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::{Json, extract::Path, extract::State};
use std::sync::Arc;

fn state_with_mock(mock_cache: MockCacheExecutor) -> AppState {
    AppState {
        cache_pool: Arc::new(CachePool::Mock(mock_cache)),
    }
}

#[tokio::test]
async fn test_create_cache_entry_created() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_create_entry()
        .times(1)
        .returning(|_, _| Ok(true));

    let response = create_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
        Json(CacheValue {
            value: "hello".to_string(),
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_cache_entry_conflict() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_create_entry()
        .times(1)
        .returning(|_, _| Ok(false));

    let response = create_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
        Json(CacheValue {
            value: "hello".to_string(),
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_cache_entry_error() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_create_entry()
        .times(1)
        .returning(|_, _| Err(anyhow::anyhow!("Redis error")));

    let response = create_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
        Json(CacheValue {
            value: "hello".to_string(),
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_get_cache_entry_found() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_get_entry()
        .times(1)
        .returning(|_| Ok(Some("hello".to_string())));

    let response = get_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_cache_entry_not_found() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_get_entry()
        .times(1)
        .returning(|_| Ok(None));

    let response = get_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_cache_entry_updated() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_update_entry()
        .times(1)
        .returning(|_, _| Ok(true));

    let response = update_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
        Json(CacheValue {
            value: "hi".to_string(),
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_cache_entry_not_found() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_update_entry()
        .times(1)
        .returning(|_, _| Ok(false));

    let response = update_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
        Json(CacheValue {
            value: "hi".to_string(),
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_cache_entry_deleted() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_delete_entry()
        .times(1)
        .returning(|_| Ok(true));

    let response = delete_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_cache_entry_not_found() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_delete_entry()
        .times(1)
        .returning(|_| Ok(false));

    let response = delete_cache_entry(
        State(state_with_mock(mock_cache)),
        Path("greeting".to_string()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
