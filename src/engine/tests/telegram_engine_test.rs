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
async fn test_get_updates_sends_offset_and_limit() {
    let server = MockServer::start_async().await;
    let get_updates = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/getUpdates")
                .json_body_includes(r#"{"offset": 8, "timeout": 25, "limit": 100}"#);
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

// ---------------------------------------------------------------------------------------
// EX0037: sender and language, photos, typed errors, update limit
// ---------------------------------------------------------------------------------------
use crate::domain::telegram::{PhotoSource, TelegramError, Update};
use crate::engine::telegram_engine::{multipart_body, update_limit_from};

#[test]
fn test_update_from_has_the_language() {
    let json = r#"{"update_id":10,"message":{"message_id":5,"from":{"id":111,"is_bot":false,"first_name":"Nick","username":"nick","language_code":"pt-br"},"chat":{"id":111,"type":"private"},"text":"/ping"}}"#;

    let update: Update = serde_json::from_str(json).unwrap();

    let from = update.from().unwrap();
    assert_eq!(from.id, 111);
    assert_eq!(from.username.as_deref(), Some("nick"));
    assert_eq!(from.language_code.as_deref(), Some("pt-br"));
    assert_eq!(update.chat_id(), Some(111));
    assert_eq!(update.message.unwrap().chat.kind, "private");
}

#[test]
fn test_button_tap_from_and_chat() {
    let json = r#"{"update_id":11,"callback_query":{"id":"q1","from":{"id":222,"is_bot":false,"first_name":"Ana","language_code":"en"},"data":"choose:yes","message":{"message_id":43,"chat":{"id":-100123,"type":"supergroup"},"text":"Pick one:"}}}"#;

    let update: Update = serde_json::from_str(json).unwrap();

    assert_eq!(update.from().unwrap().id, 222);
    // The chat is the group the button's message is in, not the user.
    assert_eq!(update.chat_id(), Some(-100123));
}

#[tokio::test]
async fn test_send_photo_by_file_id_returns_the_largest_size() {
    let server = MockServer::start_async().await;
    let send = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/sendPhoto")
                .json_body_includes(r#"{"chat_id": 5, "photo": "AgAD-small", "caption": "Fusca"}"#);
            then.status(200).json_body(json!({
                "ok": true,
                "result": {"message_id": 9, "photo": [
                    {"file_id": "small", "width": 90, "height": 60},
                    {"file_id": "big", "width": 1280, "height": 853}
                ]}
            }));
        })
        .await;

    let file_id = api_for(&server)
        .send_photo(
            5,
            PhotoSource::FileId("AgAD-small".to_string()),
            Some("Fusca".to_string()),
            None,
        )
        .await
        .unwrap();

    send.assert_async().await;
    assert_eq!(file_id, "big");
}

#[tokio::test]
async fn test_send_photo_uploads_bytes_as_multipart() {
    let server = MockServer::start_async().await;
    let upload = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/sendPhoto")
                .header_includes("content-type", "multipart/form-data; boundary=")
                .body_includes("name=\"chat_id\"\r\n\r\n5\r\n")
                .body_includes("filename=\"fusca.png\"")
                .body_includes("PNG-BYTES");
            then.status(200).json_body(json!({
                "ok": true,
                "result": {"message_id": 9, "photo": [{"file_id": "new-id", "width": 10, "height": 10}]}
            }));
        })
        .await;

    let file_id = api_for(&server)
        .send_photo(
            5,
            PhotoSource::Bytes {
                name: "fusca.png".to_string(),
                data: b"PNG-BYTES".to_vec(),
            },
            None,
            None,
        )
        .await
        .unwrap();

    upload.assert_async().await;
    assert_eq!(file_id, "new-id");
}

#[test]
fn test_multipart_body_shape() {
    let (content_type, body) = multipart_body(
        &[("chat_id".to_string(), "5".to_string())],
        "photo",
        "a\"b.png",
        b"DATA",
    );
    let boundary = content_type.split("boundary=").nth(1).unwrap();
    let text = String::from_utf8(body).unwrap();
    assert!(text.starts_with(&format!("--{boundary}\r\n")));
    assert!(text.ends_with(&format!("\r\n--{boundary}--\r\n")));
    // A quote in the name can't break the header.
    assert!(text.contains("filename=\"a_b.png\""));
}

#[tokio::test]
async fn test_rate_limit_and_blocked_user_are_typed_errors() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/sendMessage")
                .json_body_includes(r#"{"chat_id": 1}"#);
            then.status(429).json_body(json!({
                "ok": false, "error_code": 429,
                "description": "Too Many Requests: retry after 3",
                "parameters": {"retry_after": 3}
            }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/sendMessage")
                .json_body_includes(r#"{"chat_id": 2}"#);
            then.status(403).json_body(json!({
                "ok": false, "error_code": 403,
                "description": "Forbidden: bot was blocked by the user"
            }));
        })
        .await;
    let api = api_for(&server);

    let limited = api
        .send_message(1, "hi".to_string(), None)
        .await
        .unwrap_err();
    let blocked = api
        .send_message(2, "hi".to_string(), None)
        .await
        .unwrap_err();

    assert_eq!(
        limited.downcast_ref::<TelegramError>(),
        Some(&TelegramError::RateLimited {
            method: "sendMessage".to_string(),
            retry_after: 3
        })
    );
    assert!(matches!(
        blocked.downcast_ref::<TelegramError>(),
        Some(TelegramError::Forbidden { .. })
    ));
    // Still readable as text, without the token.
    assert!(!blocked.to_string().contains("TEST-TOKEN"));
}

#[test]
fn test_update_limit_from_env() {
    assert_eq!(update_limit_from(None), 100);
    assert_eq!(update_limit_from(Some("1".to_string())), 1);
}

#[test]
#[should_panic(expected = "TELEGRAM_UPDATE_LIMIT must be a number from 1 to 100.")]
fn test_update_limit_above_100_panics() {
    update_limit_from(Some("101".to_string()));
}

// EX0040: editing a message can keep (replace) its buttons, e.g. a page of a list.
#[tokio::test]
async fn test_edit_message_text_with_and_without_keyboard() {
    let server = MockServer::start_async().await;
    let with_keyboard = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/editMessageText")
                .json_body(json!({
                    "chat_id": 5, "message_id": 9, "text": "Page 2",
                    "reply_markup": {"inline_keyboard": [[{"text": "◀", "callback_data": "page:1"}]]}
                }));
            then.status(200)
                .json_body(json!({"ok": true, "result": {"message_id": 9}}));
        })
        .await;
    let without = server
        .mock_async(|when, then| {
            when.method("POST")
                .path("/bot123:TEST-TOKEN/editMessageText")
                .json_body(json!({"chat_id": 5, "message_id": 9, "text": "Done"}));
            then.status(200)
                .json_body(json!({"ok": true, "result": {"message_id": 9}}));
        })
        .await;

    let keyboard = InlineKeyboardMarkup {
        inline_keyboard: vec![vec![InlineKeyboardButton {
            text: "◀".to_string(),
            callback_data: "page:1".to_string(),
        }]],
    };
    let api = api_for(&server);
    api.edit_message_text(5, 9, "Page 2".to_string(), Some(keyboard))
        .await
        .unwrap();
    api.edit_message_text(5, 9, "Done".to_string(), None)
        .await
        .unwrap();

    with_keyboard.assert_async().await;
    without.assert_async().await;
}
