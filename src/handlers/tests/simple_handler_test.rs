use crate::handlers::simple_handler::*;
use axum::body::HttpBody;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use httpmock::prelude::*;
use axum::body::to_bytes;

#[tokio::test]
async fn test_get_pong() {
    let response = get_pong().await;
    let body = to_bytes(response.into_body(), usize::MAX).await;

    match body {
        Err(e) => panic!("Error: {}", e),
        Ok(b) => assert_eq!(&b[..], b"PONG!"),
    }
}

#[tokio::test]
async fn test_call_external_service_ok() {
    let server = MockServer::start_async().await;

    let hello_mock = server
        .mock_async(|when, then| {
            when.method("GET").path("/pong");
            then.status(200)
                .header("content-type", "text/html; charset=UTF-8")
                .body(r#"{"code": 200, "message_text": "PONG"}"#);
        })
        .await;

    unsafe {
        std::env::set_var("EXTERNAL_SERVICE_URL", server.url(""));
    }

    let response = call_external_service().await;

    hello_mock.assert();

    assert_eq!(response.into_response().status(), StatusCode::OK);
}

#[tokio::test]
async fn test_call_external_service_fail() {
    unsafe {
        std::env::set_var("EXTERNAL_SERVICE_URL", "localhost:99999");
    }

    let response = call_external_service().await;

    assert_ne!(response.into_response().status(), StatusCode::OK);
}
