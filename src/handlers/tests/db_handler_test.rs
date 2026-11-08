use crate::domain::database::{NewUser, NewUserPair, User};
use crate::engine::db_engine::{DbPool, MockDatabaseExecutor};
use crate::handlers::db_handler::*;
use crate::state::AppState;
use anyhow::anyhow;
use axum::http::StatusCode;
use axum::{Json, extract::State};
use mockall::predicate::*;
use std::sync::Arc;

#[tokio::test]
async fn test_get_users_success() {
    let mut mock_executor = MockDatabaseExecutor::new();
    let mock_users = vec![
        User {
            uid: 1,
            name: "Test User 1".to_string(),
        },
        User {
            uid: 2,
            name: "Test User 2".to_string(),
        },
    ];

    mock_executor
        .expect_execute_get_users()
        .times(1)
        .returning(move || Ok(mock_users.clone()));

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_executor)),
    };

    let response = get_users(State(state)).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_users_error() {
    let mut mock_executor = MockDatabaseExecutor::new();

    mock_executor
        .expect_execute_get_users()
        .times(1)
        .returning(|| Err(anyhow!("Database error")));

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_executor)),
    };

    let response = get_users(State(state)).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_create_user_success() {
    let mut mock_executor = MockDatabaseExecutor::new();
    let new_user = NewUser {
        name: "Test User".to_string(),
    };

    mock_executor
        .expect_execute_create_user()
        .with(eq("Test User".to_string()))
        .times(1)
        .returning(|_| Ok("OK".to_string()));

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_executor)),
    };

    let response = create_user(State(state), Json(new_user)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_user_error() {
    let mut mock_executor = MockDatabaseExecutor::new();
    let new_user = NewUser {
        name: "Test User".to_string(),
    };

    mock_executor
        .expect_execute_create_user()
        .times(1)
        .returning(|_| Err(anyhow!("Database error")));

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_executor)),
    };

    let response = create_user(State(state), Json(new_user)).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_create_user_pair_success() {
    let mut mock_executor = MockDatabaseExecutor::new();
    let pair = NewUserPair {
        first: "Ann".to_string(),
        second: "Bob".to_string(),
    };

    mock_executor
        .expect_execute_create_user_pair()
        .with(eq("Ann".to_string()), eq("Bob".to_string()))
        .times(1)
        .returning(|_, _| Ok("OK".to_string()));

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_executor)),
    };

    let response = create_user_pair(State(state), Json(pair)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_user_pair_error() {
    let mut mock_executor = MockDatabaseExecutor::new();
    let pair = NewUserPair {
        first: "Cid".to_string(),
        second: "NameLongerThan15".to_string(),
    };

    mock_executor
        .expect_execute_create_user_pair()
        .times(1)
        .returning(|_, _| Err(anyhow!("value too long for type character varying(15)")));

    let state = AppState {
        db_pool: Arc::new(DbPool::Mock(mock_executor)),
    };

    let response = create_user_pair(State(state), Json(pair)).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
