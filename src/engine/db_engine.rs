use crate::domain::database::User;
use crate::engine::outbox_engine::{insert_outbox_event, wake_outbox_worker};
use crate::state::AppState;
use anyhow::{Result, anyhow};
use axum::extract::State;
#[cfg(test)]
use mockall::automock;
use sqlx::{MySql, Pool, Row, query};

// Wrapper type that can be either a real pool or a mock (in tests)
pub enum DbPool {
    Real(Pool<MySql>),
    #[cfg(test)]
    Mock(MockDatabaseExecutor),
}

#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait DatabaseExecutor: Send + Sync {
    async fn execute_get_users(&self) -> Result<Vec<User>>;
    async fn execute_create_user(&self, name: String) -> Result<String>;
    async fn execute_create_demo_event(&self, msg: String) -> Result<String>;
}

#[async_trait::async_trait]
impl DatabaseExecutor for Pool<MySql> {
    async fn execute_get_users(&self) -> Result<Vec<User>> {
        let users = query("CALL sp_Return_USERS();")
            .map(|row: sqlx::mysql::MySqlRow| User {
                uid: row.get(0),
                name: row.get(1),
            })
            .fetch_all(self)
            .await?;
        Ok(users)
    }

    async fn execute_create_user(&self, name: String) -> Result<String> {
        let _ = query("CALL sp_Insert_User(?)")
            .bind(name)
            .execute(self)
            .await?;
        Ok("OK".to_string())
    }

    // Demo for the outbox (OB-1): the event is written inside a transaction. Real code
    // would also change its own data in that same transaction, before the event.
    async fn execute_create_demo_event(&self, msg: String) -> Result<String> {
        let mut tx = self.begin().await?;
        insert_outbox_event(&mut tx, "DemoEvent", &serde_json::json!({ "msg": msg })).await?;
        if msg == "rollback" {
            // Fails on purpose AFTER the insert: dropping `tx` rolls the event back,
            // so it never exists and the worker never sees it.
            return Err(anyhow!("demo failure after insert, rolled back"));
        }
        tx.commit().await?;
        // Only after the commit: waking the worker earlier would find nothing to do.
        wake_outbox_worker();
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

    async fn execute_create_demo_event(&self, msg: String) -> Result<String> {
        match self {
            DbPool::Real(pool) => pool.execute_create_demo_event(msg).await,
            #[cfg(test)]
            DbPool::Mock(mock) => mock.execute_create_demo_event(msg).await,
        }
    }
}

pub async fn get_users_db_call(State(state): State<AppState>) -> Result<Vec<User>> {
    state.db_pool.execute_get_users().await
}

pub async fn create_user_db_call(State(state): State<AppState>, name: String) -> Result<String> {
    state.db_pool.execute_create_user(name).await
}

pub async fn create_demo_event_db_call(
    State(state): State<AppState>,
    msg: String,
) -> Result<String> {
    state.db_pool.execute_create_demo_event(msg).await
}
