use crate::state::AppState;
use anyhow::Result;
use axum::extract::State;
#[cfg(test)]
use mockall::automock;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, ExistenceCheck, SetOptions};

// Namespaced so these generic key/value demo entries don't collide with any other use
// of this Redis instance.
const DEMO_ENTRY_PREFIX: &str = "demo:";

fn namespaced_key(key: &str) -> String {
    format!("{DEMO_ENTRY_PREFIX}{key}")
}

// Wrapper type that can be either a real Redis connection manager or a mock (in tests)
pub enum CachePool {
    Real(ConnectionManager),
    #[cfg(test)]
    Mock(MockCacheExecutor),
}

#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait CacheExecutor: Send + Sync {
    /// Sets `key` only if it doesn't already exist. Returns `false` (no-op) if it does.
    async fn execute_create_entry(&self, key: String, value: String) -> Result<bool>;
    async fn execute_get_entry(&self, key: String) -> Result<Option<String>>;
    /// Sets `key` only if it already exists. Returns `false` (no-op) if it doesn't.
    async fn execute_update_entry(&self, key: String, value: String) -> Result<bool>;
    /// Returns `false` if `key` didn't exist.
    async fn execute_delete_entry(&self, key: String) -> Result<bool>;
}

#[async_trait::async_trait]
impl CacheExecutor for ConnectionManager {
    async fn execute_create_entry(&self, key: String, value: String) -> Result<bool> {
        let mut conn = self.clone();
        let options = SetOptions::default().conditional_set(ExistenceCheck::NX);
        let result: Option<String> = conn
            .set_options(namespaced_key(&key), value, options)
            .await?;
        Ok(result.is_some())
    }

    async fn execute_get_entry(&self, key: String) -> Result<Option<String>> {
        let mut conn = self.clone();
        let value: Option<String> = conn.get(namespaced_key(&key)).await?;
        Ok(value)
    }

    async fn execute_update_entry(&self, key: String, value: String) -> Result<bool> {
        let mut conn = self.clone();
        let options = SetOptions::default().conditional_set(ExistenceCheck::XX);
        let result: Option<String> = conn
            .set_options(namespaced_key(&key), value, options)
            .await?;
        Ok(result.is_some())
    }

    async fn execute_delete_entry(&self, key: String) -> Result<bool> {
        let mut conn = self.clone();
        let deleted: usize = conn.del(namespaced_key(&key)).await?;
        Ok(deleted > 0)
    }
}

#[async_trait::async_trait]
impl CacheExecutor for CachePool {
    async fn execute_create_entry(&self, key: String, value: String) -> Result<bool> {
        match self {
            CachePool::Real(conn) => conn.execute_create_entry(key, value).await,
            #[cfg(test)]
            CachePool::Mock(mock) => mock.execute_create_entry(key, value).await,
        }
    }

    async fn execute_get_entry(&self, key: String) -> Result<Option<String>> {
        match self {
            CachePool::Real(conn) => conn.execute_get_entry(key).await,
            #[cfg(test)]
            CachePool::Mock(mock) => mock.execute_get_entry(key).await,
        }
    }

    async fn execute_update_entry(&self, key: String, value: String) -> Result<bool> {
        match self {
            CachePool::Real(conn) => conn.execute_update_entry(key, value).await,
            #[cfg(test)]
            CachePool::Mock(mock) => mock.execute_update_entry(key, value).await,
        }
    }

    async fn execute_delete_entry(&self, key: String) -> Result<bool> {
        match self {
            CachePool::Real(conn) => conn.execute_delete_entry(key).await,
            #[cfg(test)]
            CachePool::Mock(mock) => mock.execute_delete_entry(key).await,
        }
    }
}

pub async fn create_entry_call(
    State(state): State<AppState>,
    key: String,
    value: String,
) -> Result<bool> {
    state.cache_pool.execute_create_entry(key, value).await
}

pub async fn get_entry_call(State(state): State<AppState>, key: String) -> Result<Option<String>> {
    state.cache_pool.execute_get_entry(key).await
}

pub async fn update_entry_call(
    State(state): State<AppState>,
    key: String,
    value: String,
) -> Result<bool> {
    state.cache_pool.execute_update_entry(key, value).await
}

pub async fn delete_entry_call(State(state): State<AppState>, key: String) -> Result<bool> {
    state.cache_pool.execute_delete_entry(key).await
}
