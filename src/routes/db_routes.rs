// The MariaDB pack's routes (plugged in with one `.merge(...)` line in routes/mod.rs).
use crate::handlers::db_handler::{create_user, get_users};
use crate::state::AppState;
use axum::Router;
use axum::routing::{get, post};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(get_users))
        .route("/users", post(create_user))
}
