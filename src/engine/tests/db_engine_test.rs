use crate::domain::database::User;
use crate::engine::db_engine::*;
use crate::state::AppState;
use axum::extract::State;
use std::sync::Arc;

#[tokio::test]
async fn test_get_users_db_call() {
    let mut mock_db = MockDatabaseExecutor::new();

    // Setup mock expectations
    mock_db.expect_execute_get_users().times(1).returning(|| {
        Ok(vec![User {
            uid: 1,
            name: "Test User".to_string(),
        }])
    });

    // Create AppState with our mock wrapped in DbPool
    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_db)),
        ..Default::default()
    };

    // Test the actual function
    let result = get_users_db_call(State(state)).await.unwrap();

    // Assertions
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "Test User");
    assert_eq!(result[0].uid, 1);
}

#[tokio::test]
async fn test_create_user_db_call() {
    let mut mock_db = MockDatabaseExecutor::new();

    // Setup mock expectations
    mock_db
        .expect_execute_create_user()
        .with(mockall::predicate::eq("Test User".to_string()))
        .times(1)
        .returning(|_| Ok("OK".to_string()));

    // Create AppState with our mock wrapped in DbPool
    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_db)),
        ..Default::default()
    };

    // Test the actual function
    let result = create_user_db_call(State(state), "Test User".to_string())
        .await
        .unwrap();

    // Assertions
    assert_eq!(result, "OK");
}

#[tokio::test]
async fn test_create_user_pair_db_call() {
    let mut mock_db = MockDatabaseExecutor::new();

    mock_db
        .expect_execute_create_user_pair()
        .with(
            mockall::predicate::eq("Ann".to_string()),
            mockall::predicate::eq("Bob".to_string()),
        )
        .times(1)
        .returning(|_, _| Ok("OK".to_string()));

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_db)),
        ..Default::default()
    };

    let result = create_user_pair_db_call(State(state), "Ann".to_string(), "Bob".to_string())
        .await
        .unwrap();

    assert_eq!(result, "OK");
}

#[tokio::test]
async fn test_create_user_pair_db_call_propagates_error() {
    let mut mock_db = MockDatabaseExecutor::new();

    mock_db
        .expect_execute_create_user_pair()
        .times(1)
        .returning(|_, _| {
            Err(anyhow::anyhow!(
                "value too long for type character varying(15)"
            ))
        });

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_db)),
        ..Default::default()
    };

    let result = create_user_pair_db_call(
        State(state),
        "Cid".to_string(),
        "NameLongerThan15".to_string(),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "value too long for type character varying(15)"
    );
}
