// The http-external-call pack: calls another service (EXTERNAL_SERVICE_URL) with reqwest.
use crate::domain::external_call::Message;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::{error, info, warn};

pub async fn call_external_service() -> Response {
    info!("call_external_service called");
    let base_url = std::env::var("EXTERNAL_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());

    let url = format!("{}/pong", base_url);
    let response = reqwest::get(url).await;
    info!("another microservice called");

    match response {
        Ok(resp) => {
            warn!("Ok response from external service");
            let json: Message = resp.json().await.unwrap_or(Message {
                code: 400,
                message_text: "Failed to parse external response".into(),
            });
            (
                StatusCode::from_u16(json.code as u16).unwrap_or(StatusCode::EXPECTATION_FAILED),
                format!("{} !!", json.message_text),
            )
                .into_response()
        }
        Err(err) => {
            error!("Error response from external service: {}", err);
            (
                StatusCode::EXPECTATION_FAILED,
                "Failed to reach external service",
            )
                .into_response()
        }
    }
}
