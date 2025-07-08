use crate::domain::general::Message;
use serde_json::from_str;

#[test]
fn test_message_default() {
    let message = Message::default();
    assert_eq!(message.code, 0);
    assert_eq!(message.message_text, "");
}

#[test]
fn test_message_serialization() {
    let message = Message {
        code: 200,
        message_text: String::from("Success"),
    };

    let serialized = serde_json::to_string(&message).unwrap();
    let expected = r#"{"code":200,"message_text":"Success"}"#;
    assert_eq!(serialized, expected);
}

#[test]
fn test_message_deserialization() {
    let json = r#"{"code":404,"message_text":"Not Found"}"#;
    let message: Message = from_str(json).unwrap();

    assert_eq!(message.code, 404);
    assert_eq!(message.message_text, "Not Found");
}
