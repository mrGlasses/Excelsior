use crate::domain::general::{FilterParams, Message, Params};
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

#[test]
fn test_params_deserialization() {
    let json = r#"{"param_1":42,"param_2":"test"}"#;
    let params: Params = from_str(json).unwrap();

    assert_eq!(params.param_1, 42);
    assert_eq!(params.param_2, "test");
}

#[test]
fn test_filter_params_with_all_fields() {
    let json = r#"{"name":"John","age":30,"active":true}"#;
    let filter: FilterParams = from_str(json).unwrap();

    assert_eq!(filter.name, Some(String::from("John")));
    assert_eq!(filter.age, Some(30));
    assert_eq!(filter.active, Some(true));
}

#[test]
fn test_filter_params_with_partial_fields() {
    let json = r#"{"name":"John","age":null,"active":null}"#;
    let filter: FilterParams = from_str(json).unwrap();

    assert_eq!(filter.name, Some(String::from("John")));
    assert_eq!(filter.age, None);
    assert_eq!(filter.active, None);
}

#[test]
fn test_filter_params_empty() {
    let json = r#"{"name":null,"age":null,"active":null}"#;
    let filter: FilterParams = from_str(json).unwrap();

    assert_eq!(filter.name, None);
    assert_eq!(filter.age, None);
    assert_eq!(filter.active, None);
}
