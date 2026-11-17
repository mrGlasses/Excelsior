// The PostgreSQL pack's routes (plugged in with one `.merge(...)` line in routes/mod.rs).
use crate::handlers::db_handler::{create_user, create_user_pair, get_users};
use crate::state::AppState;
use axum::Router;
use axum::routing::{get, post};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(get_users))
        .route("/users", post(create_user))
        .route("/users/pair", post(create_user_pair))
}
