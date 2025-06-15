use crate::handlers::simple_handler::*;
use crate::{handlers::db_handler::*, state::AppState};
use axum::routing::post;
use axum::{routing::get, Router};
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing::Span;

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/ping", get(get_pong))
        .route("/users", get(get_users))
        .route("/users", post(create_user))
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
        ))
        .with_state(state)
}

//more than 1 route file? search for "axum merge routes"
