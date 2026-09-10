use crate::domain::database::User;
use crate::state::AppState;
use anyhow::Result;
use axum::extract::State;
#[cfg(test)]
use mockall::automock;
use sqlx::{Pool, Postgres, Row, query};

// Wrapper type that can be either a real pool or a mock (in tests)
pub enum DbPool {
    Real(Pool<Postgres>),
    #[cfg(test)]
    Mock(MockDatabaseExecutor),
}

#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait DatabaseExecutor: Send + Sync {
    async fn execute_get_users(&self) -> Result<Vec<User>>;
    async fn execute_create_user(&self, name: String) -> Result<String>;
}

#[async_trait::async_trait]
impl DatabaseExecutor for Pool<Postgres> {
    async fn execute_get_users(&self) -> Result<Vec<User>> {
        let users = query("SELECT * FROM sp_return_users();")
            .map(|row: sqlx::postgres::PgRow| User {
                uid: row.get(0),
                name: row.get(1),
            })
            .fetch_all(self)
            .await?;
        Ok(users)
    }

    async fn execute_create_user(&self, name: String) -> Result<String> {
        let _ = query("CALL sp_insert_user($1)")
            .bind(name)
            .execute(self)
            .await?;
        Ok("OK".to_string())
    }
}

#[async_trait::async_trait]
impl DatabaseExecutor for DbPool {
    async fn execute_get_users(&self) -> Result<Vec<User>> {
        match self {
            DbPool::Real(pool) => pool.execute_get_users().await,
            #[cfg(test)]
            DbPool::Mock(mock) => mock.execute_get_users().await,
        }
    }

    async fn execute_create_user(&self, name: String) -> Result<String> {
        match self {
            DbPool::Real(pool) => pool.execute_create_user(name).await,
            #[cfg(test)]
            DbPool::Mock(mock) => mock.execute_create_user(name).await,
        }
    }
}

pub async fn get_users_db_call(State(state): State<AppState>) -> Result<Vec<User>> {
    state.db_pool.execute_get_users().await
}

pub async fn create_user_db_call(State(state): State<AppState>, name: String) -> Result<String> {
    state.db_pool.execute_create_user(name).await
}
