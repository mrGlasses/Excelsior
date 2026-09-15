use crate::engine::cache_engine::*;
use crate::state::AppState;
use axum::extract::State;
use std::sync::Arc;

fn state_with_mock(mock_cache: MockCacheExecutor) -> AppState {
    AppState {
        cache_pool: Arc::new(CachePool::Mock(mock_cache)),
    }
}

#[tokio::test]
async fn test_create_entry_call_created() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_create_entry()
        .withf(|key, value| key == "greeting" && value == "hello")
        .times(1)
        .returning(|_, _| Ok(true));

    let result = create_entry_call(
        State(state_with_mock(mock_cache)),
        "greeting".to_string(),
        "hello".to_string(),
    )
    .await
    .unwrap();

    assert!(result);
}

#[tokio::test]
async fn test_create_entry_call_already_exists() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_create_entry()
        .times(1)
        .returning(|_, _| Ok(false));

    let result = create_entry_call(
        State(state_with_mock(mock_cache)),
        "greeting".to_string(),
        "hello".to_string(),
    )
    .await
    .unwrap();

    assert!(!result);
}

#[tokio::test]
async fn test_get_entry_call_found() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_get_entry()
        .withf(|key| key == "greeting")
        .times(1)
        .returning(|_| Ok(Some("hello".to_string())));

    let result = get_entry_call(State(state_with_mock(mock_cache)), "greeting".to_string())
        .await
        .unwrap();

    assert_eq!(result, Some("hello".to_string()));
}

#[tokio::test]
async fn test_get_entry_call_missing() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_get_entry()
        .times(1)
        .returning(|_| Ok(None));

    let result = get_entry_call(State(state_with_mock(mock_cache)), "greeting".to_string())
        .await
        .unwrap();

    assert_eq!(result, None);
}

#[tokio::test]
async fn test_update_entry_call_updated() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_update_entry()
        .withf(|key, value| key == "greeting" && value == "hi")
        .times(1)
        .returning(|_, _| Ok(true));

    let result = update_entry_call(
        State(state_with_mock(mock_cache)),
        "greeting".to_string(),
        "hi".to_string(),
    )
    .await
    .unwrap();

    assert!(result);
}

#[tokio::test]
async fn test_update_entry_call_missing() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_update_entry()
        .times(1)
        .returning(|_, _| Ok(false));

    let result = update_entry_call(
        State(state_with_mock(mock_cache)),
        "greeting".to_string(),
        "hi".to_string(),
    )
    .await
    .unwrap();

    assert!(!result);
}

#[tokio::test]
async fn test_delete_entry_call_deleted() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_delete_entry()
        .withf(|key| key == "greeting")
        .times(1)
        .returning(|_| Ok(true));

    let result = delete_entry_call(State(state_with_mock(mock_cache)), "greeting".to_string())
        .await
        .unwrap();

    assert!(result);
}

#[tokio::test]
async fn test_delete_entry_call_missing() {
    let mut mock_cache = MockCacheExecutor::new();
    mock_cache
        .expect_execute_delete_entry()
        .times(1)
        .returning(|_| Ok(false));

    let result = delete_entry_call(State(state_with_mock(mock_cache)), "greeting".to_string())
        .await
        .unwrap();

    assert!(!result);
}
