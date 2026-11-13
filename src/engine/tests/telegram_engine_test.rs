use crate::domain::telegram::{InlineKeyboardButton, InlineKeyboardMarkup};
use crate::engine::telegram_engine::{TelegramApi, TelegramExecutor};
use httpmock::prelude::*;
use serde_json::json;

const TOKEN: &str = "123:TEST-TOKEN";

fn api_for(server: &MockServer) -> TelegramApi {
    TelegramApi::new(&server.base_url(), TOKEN, 25).unwrap()
}

#[tokio::test]
async fn test_get_me() {
    let server = MockServer::start_async().await;
    let get_me = server
        .mock_async(|when, then| {
            when.method("POST").path("/bot123:TEST-TOKEN/getMe");
            then.status(200).json_body(json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "first_name": "Card", "username": "CardBot"}
            }));
        })
        .await;

    let me = api_for(&server).get_me().await.unwrap();

    get_me.assert_async().await;
    assert_eq!(me.username.as_deref(), Some("CardBot"));
}

#[tokio::test]
async fn test_get_updates_sends_offset_and_one_at_a_time() {
    let server = MockServer::start_async().await;
    let get_updates = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/getUpdates")
                .json_body_includes(r#"{"offset": 8, "timeout": 25, "limit": 1}"#);
            then.status(200).json_body(json!({
                "ok": true,
                "result": [{"update_id": 8, "message": {"message_id": 1, "chat": {"id": 5}, "text": "/ping"}}]
            }));
        })
        .await;

    let updates = api_for(&server).get_updates(Some(8), 25).await.unwrap();

    get_updates.assert_async().await;
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].update_id, 8);
}

#[tokio::test]
async fn test_send_message_with_keyboard() {
    let server = MockServer::start_async().await;
    let send = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/sendMessage")
                .json_body(json!({
                    "chat_id": 5,
                    "text": "Pick one:",
                    "reply_markup": {"inline_keyboard": [[{"text": "Yes", "callback_data": "choose:yes"}]]}
                }));
            then.status(200)
                .json_body(json!({"ok": true, "result": {"message_id": 2}}));
        })
        .await;

    let keyboard = InlineKeyboardMarkup {
        inline_keyboard: vec![vec![InlineKeyboardButton {
            text: "Yes".to_string(),
            callback_data: "choose:yes".to_string(),
        }]],
    };
    api_for(&server)
        .send_message(5, "Pick one:".to_string(), Some(keyboard))
        .await
        .unwrap();

    send.assert_async().await;
}

#[tokio::test]
async fn test_telegram_error_becomes_err_with_description() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method("POST").path("/bot123:TEST-TOKEN/getMe");
            then.status(401)
                .json_body(json!({"ok": false, "error_code": 401, "description": "Unauthorized"}));
        })
        .await;

    let err = api_for(&server).get_me().await.unwrap_err();

    assert_eq!(err.to_string(), "telegram getMe failed: Unauthorized");
}

#[tokio::test]
async fn test_network_error_does_not_leak_the_token() {
    // Port 9 (discard) on localhost: nothing listens, so the request fails to connect.
    let api = TelegramApi::new("http://127.0.0.1:9", TOKEN, 25).unwrap();

    let err = api.get_me().await.unwrap_err();

    let everything_printed = format!("{:?} {:#}", err, err);
    assert!(
        !everything_printed.contains("TEST-TOKEN"),
        "token leaked into the error: {}",
        everything_printed
    );
}
