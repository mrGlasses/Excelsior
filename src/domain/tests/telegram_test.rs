use crate::domain::telegram::*;

#[test]
fn test_message_update_parses_and_ignores_extra_fields() {
    // Shape of a real update; "from", "date" etc. aren't in our structs and are ignored.
    let json = r#"{
        "update_id": 7,
        "message": {
            "message_id": 42,
            "from": {"id": 99, "is_bot": false, "first_name": "Nick"},
            "chat": {"id": 99, "type": "private"},
            "date": 1762750000,
            "text": "/ping"
        }
    }"#;

    let update: Update = serde_json::from_str(json).unwrap();

    assert_eq!(update.update_id, 7);
    let message = update.message.unwrap();
    assert_eq!(message.message_id, 42);
    assert_eq!(message.chat.id, 99);
    assert_eq!(message.text.as_deref(), Some("/ping"));
    assert!(update.callback_query.is_none());
}

#[test]
fn test_callback_query_update_parses() {
    let json = r#"{
        "update_id": 8,
        "callback_query": {
            "id": "4382bfdwdsb323b2d9",
            "from": {"id": 99, "is_bot": false, "first_name": "Nick"},
            "message": {"message_id": 43, "chat": {"id": 99, "type": "private"}, "date": 1, "text": "Pick one:"},
            "chat_instance": "123",
            "data": "choose:yes"
        }
    }"#;

    let update: Update = serde_json::from_str(json).unwrap();

    let query = update.callback_query.unwrap();
    assert_eq!(query.id, "4382bfdwdsb323b2d9");
    assert_eq!(query.data.as_deref(), Some("choose:yes"));
    assert_eq!(query.message.unwrap().message_id, 43);
}

#[test]
fn test_error_response_parses() {
    let json = r#"{"ok": false, "error_code": 401, "description": "Unauthorized"}"#;

    let response: TelegramResponse<BotUser> = serde_json::from_str(json).unwrap();

    assert!(!response.ok);
    assert!(response.result.is_none());
    assert_eq!(response.description.as_deref(), Some("Unauthorized"));
}

#[test]
fn test_keyboard_serializes_as_telegram_expects() {
    let keyboard = InlineKeyboardMarkup {
        inline_keyboard: vec![vec![InlineKeyboardButton {
            text: "Yes".to_string(),
            callback_data: "choose:yes".to_string(),
        }]],
    };

    assert_eq!(
        serde_json::to_value(keyboard).unwrap(),
        serde_json::json!({"inline_keyboard": [[{"text": "Yes", "callback_data": "choose:yes"}]]})
    );
}
