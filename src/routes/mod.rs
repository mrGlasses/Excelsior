use crate::handlers::cache_handler::*;
use crate::handlers::simple_handler::get_pong;
use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use http::StatusCode;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing::Span;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/ping", get(get_pong))
        // Generic Redis CRUD demo: localhost/cache/some-key with a JSON body
        // {"value": "..."} for create/update.
        .route("/cache/{key}", get(get_cache_entry))
        .route("/cache/{key}", post(create_cache_entry))
        .route("/cache/{key}", put(update_cache_entry))
        .route("/cache/{key}", delete(delete_cache_entry))
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

//more than 1 route file? search for "axum merge routes"
