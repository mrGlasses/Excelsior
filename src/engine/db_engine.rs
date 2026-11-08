use crate::domain::database::User;
use crate::state::AppState;
use anyhow::Result;
use axum::extract::State;
#[cfg(test)]
use mockall::automock;
use sqlx::{PgConnection, Pool, Postgres, Row, query};

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
    async fn execute_create_user_pair(&self, first: String, second: String) -> Result<String>;
}

// Query functions take `&mut PgConnection` instead of the pool, so the same function
// works with either:
//   - a plain connection: `insert_user(&mut *pool.acquire().await?, name)`
//   - a transaction:      `insert_user(&mut *tx, name)` (a Transaction derefs to its connection)
// Several of them sharing one transaction all commit together, or none do.
pub async fn insert_user(conn: &mut PgConnection, name: &str) -> Result<()> {
    query("CALL sp_insert_user($1)")
        .bind(name)
        .execute(conn)
        .await?;
    Ok(())
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
        let mut conn = self.acquire().await?;
        insert_user(&mut conn, &name).await?;
        Ok("OK".to_string())
    }

    async fn execute_create_user_pair(&self, first: String, second: String) -> Result<String> {
        let mut tx = self.begin().await?;
        insert_user(&mut tx, &first).await?;
        insert_user(&mut tx, &second).await?;
        // Returning early with `?` above drops `tx` without committing,
        // and sqlx rolls a dropped transaction back.
        tx.commit().await?;
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

    async fn execute_create_user_pair(&self, first: String, second: String) -> Result<String> {
        match self {
            DbPool::Real(pool) => pool.execute_create_user_pair(first, second).await,
            #[cfg(test)]
            DbPool::Mock(mock) => mock.execute_create_user_pair(first, second).await,
        }
    }
}

pub async fn get_users_db_call(State(state): State<AppState>) -> Result<Vec<User>> {
    state.db_pool.execute_get_users().await
}

pub async fn create_user_db_call(State(state): State<AppState>, name: String) -> Result<String> {
    state.db_pool.execute_create_user(name).await
}

pub async fn create_user_pair_db_call(
    State(state): State<AppState>,
    first: String,
    second: String,
) -> Result<String> {
    state.db_pool.execute_create_user_pair(first, second).await
}
