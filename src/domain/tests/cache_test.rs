use crate::domain::cache::CacheValue;
use serde_json::{from_value, json, to_value};

#[test]
fn test_cache_value_serialization() {
    let entry = CacheValue {
        value: "hello".to_string(),
    };

    let serialized = to_value(&entry).unwrap();
    let expected = json!({ "value": "hello" });

    assert_eq!(serialized, expected);
}

#[test]
fn test_cache_value_deserialization() {
    let json_data = json!({ "value": "hello" });

    let entry: CacheValue = from_value(json_data).unwrap();
    assert_eq!(entry.value, "hello");
}

#[test]
fn test_cache_value_missing_field() {
    let json_data = json!({});
    let result = from_value::<CacheValue>(json_data);
    assert!(result.is_err());
}
