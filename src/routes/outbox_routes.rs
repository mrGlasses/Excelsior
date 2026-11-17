// The PostgreSQL outbox pack's routes (plugged in with one `.merge(...)` line in routes/mod.rs).
use crate::handlers::outbox_handler::create_demo_event;
use crate::state::AppState;
use axum::Router;
use axum::routing::post;

pub fn routes() -> Router<AppState> {
    // Outbox demo: {"msg": "hi"}
    Router::new().route("/demo-events", post(create_demo_event))
}
