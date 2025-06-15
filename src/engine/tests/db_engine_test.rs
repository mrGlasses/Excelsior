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
    };

    // Test the actual function
    let result = create_user_db_call(State(state), "Test User".to_string())
        .await
        .unwrap();

    // Assertions
    assert_eq!(result, "OK");
}
