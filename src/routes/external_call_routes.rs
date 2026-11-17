// The http-external-call pack's routes (plugged in with one `.merge(...)` line in routes/mod.rs).
use crate::handlers::external_call_handler::call_external_service;
use crate::state::AppState;
use axum::Router;
use axum::routing::get;

pub fn routes() -> Router<AppState> {
    Router::new().route("/its-a-rainy-day", get(call_external_service))
}
