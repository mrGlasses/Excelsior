use crate::handlers::simple_handler::get_pong;
use crate::state::AppState;
use axum::{Router, routing::get};
use http::StatusCode;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing::Span;

// The pack registry for routes. A pack puts its routes in its own file, next to this one
// (`<pack>_routes.rs` with `pub fn routes() -> Router<AppState>`), and adds ONE
// `pub mod <pack>_routes;` line here and ONE `.merge(...)` line in create_routes().
// mod.rs files are `merge=union` in .gitattributes, so packs adding lines never conflict.
pub mod db_routes;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/ping", get(get_pong))
        .merge(db_routes::routes())
        // One line per pack, e.g. `.merge(db_routes::routes())`
        .layer((
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Micros),
                )
                .on_failure(
                    |failure_class: ServerErrorsFailureClass, latency: Duration, _: &Span| {
                        tracing::error!(
                            failure_class = ?failure_class,
                            latency = ?latency,
                            "request failed"
                        );
                    },
                ),
            CompressionLayer::new(),
            RequestBodyLimitLayer::new(1024 * 1024 * 10), // 10MB limit
            TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(60)),
        ))
        .with_state(state)
}
