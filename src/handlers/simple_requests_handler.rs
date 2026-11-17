// The simple-requests pack: demo handlers that read a header, path parameters, a query
// string and a JSON body.
use crate::domain::simple_requests::{FilterParams, Message, Params};
use axum::{
    Json,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::info;

pub async fn protected_route(headers: HeaderMap) -> Response {
    // Verifies if the specific header exists and have the correct value
    match headers.get("X-Custom-Header") {
        Some(header_value) if header_value == "secret-value" => {
            (StatusCode::OK, "Access Granted!").into_response()
        }
        _ => (StatusCode::UNAUTHORIZED, "Invalid or missing header").into_response(),
    }
}

pub async fn get_params(Path(Params { param_1, param_2 }): Path<Params>) -> Response {
    (
        StatusCode::OK,
        format!("Parameter 1: {}, Parameter 2: {}", param_1, param_2),
    )
        .into_response()
}

pub async fn get_question(Query(params): Query<FilterParams>) -> Response {
    let response = format!(
        "Filters: name={}, age={}, active={}",
        params.name.unwrap_or_default(),
        params.age.unwrap_or_default(),
        params.active.unwrap_or_default()
    );
    (StatusCode::OK, response).into_response()
}

pub async fn post_body_data(Json(payload): Json<Message>) -> Response {
    info!("Received payload: {:?}", payload);
    let response = format!(
        "Received message with code: {}, text: {}",
        payload.code, payload.message_text
    );
    (StatusCode::OK, response).into_response()
}
