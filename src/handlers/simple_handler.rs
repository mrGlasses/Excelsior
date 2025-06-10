use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::info;

pub async fn get_pong() -> Response {
    info!("PONG!");
    (StatusCode::OK, "PONG!").into_response()
}
