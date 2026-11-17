use crate::handlers::external_call_handler::call_external_service;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use httpmock::MockServer;
use serial_test::serial;

// Both tests set EXTERNAL_SERVICE_URL, so they must not run at the same time.

#[tokio::test]
#[serial]
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
#[serial]
async fn test_call_external_service_fail() {
    unsafe {
        std::env::set_var("EXTERNAL_SERVICE_URL", "localhost:99999");
    }

    let response = call_external_service().await;

    assert_ne!(response.into_response().status(), StatusCode::OK);
}
