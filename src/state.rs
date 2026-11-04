use crate::engine::cache_engine::CachePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub cache_pool: Arc<CachePool>,
}
