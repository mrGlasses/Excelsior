use crate::database::connection::{DEFAULT_MAX_CONNECTIONS, max_connections_from};

#[test]
fn test_max_connections_unset_uses_default() {
    assert_eq!(max_connections_from(None), DEFAULT_MAX_CONNECTIONS);
    assert_eq!(DEFAULT_MAX_CONNECTIONS, 5);
}

#[test]
fn test_max_connections_from_env_value() {
    assert_eq!(max_connections_from(Some("10".to_string())), 10);
}

#[test]
fn test_max_connections_ignores_surrounding_spaces() {
    assert_eq!(max_connections_from(Some(" 7 ".to_string())), 7);
}

#[test]
#[should_panic(expected = "DATABASE_MAX_CONNECTIONS must be a number greater than 0.")]
fn test_max_connections_not_a_number_panics() {
    max_connections_from(Some("ten".to_string()));
}

#[test]
#[should_panic(expected = "DATABASE_MAX_CONNECTIONS must be a number greater than 0.")]
fn test_max_connections_zero_panics() {
    max_connections_from(Some("0".to_string()));
}
