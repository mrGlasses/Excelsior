use crate::domain::i18n::{CardsParams, HelloParams};
use crate::handlers::i18n_handler::*;
use crate::utils::i18n::Locale;
use axum::body::to_bytes;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::Response;

async fn body_text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn test_hello_without_name() {
    let response = hello(Locale("pt-BR"), Query(HelloParams { name: None })).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, "Olá, mundo!");
}

#[tokio::test]
async fn test_hello_with_name() {
    let response = hello(
        Locale("en"),
        Query(HelloParams {
            name: Some("Nick".to_string()),
        }),
    )
    .await;
    assert_eq!(body_text(response).await, "Hello, Nick!");
}

#[tokio::test]
async fn test_cards_in_portuguese() {
    let response = cards(Locale("pt-BR"), Query(CardsParams { count: 3 })).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, "Você tem 3 cartas");
}
