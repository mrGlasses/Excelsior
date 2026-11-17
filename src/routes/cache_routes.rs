// The Redis pack's routes (plugged in with one `.merge(...)` line in routes/mod.rs).
// Generic Redis CRUD demo: localhost/cache/some-key with a JSON body {"value": "..."}
// for create/update.
use crate::handlers::cache_handler::{
    create_cache_entry, delete_cache_entry, get_cache_entry, update_cache_entry,
};
use crate::state::AppState;
use axum::Router;
use axum::routing::{delete, get, post, put};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cache/{key}", get(get_cache_entry))
        .route("/cache/{key}", post(create_cache_entry))
        .route("/cache/{key}", put(update_cache_entry))
        .route("/cache/{key}", delete(delete_cache_entry))
}
