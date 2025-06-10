use crate::handlers::simple_handler::*;
use axum::body::HttpBody;

#[tokio::test]
async fn test_get_pong() {
    let response = get_pong().await;
    let body = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(&body[..], b"PONG!");
}
