use crate::handlers::simple_handler::*;
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
