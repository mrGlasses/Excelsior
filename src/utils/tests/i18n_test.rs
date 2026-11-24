use crate::utils::i18n::*;
use fluent_bundle::FluentArgs;
use fluent_syntax::ast::Entry;
use std::collections::BTreeSet;

// --- I18N-1: picking the language ---

#[test]
fn test_negotiate_exact_tag() {
    assert_eq!(negotiate(Some("pt-BR"), "en"), "pt-BR");
    assert_eq!(negotiate(Some("en"), "pt-BR"), "en");
}

#[test]
fn test_negotiate_ignores_case_and_matches_the_language() {
    assert_eq!(negotiate(Some("PT-br"), "en"), "pt-BR");
    // Only the language: "pt" and "pt-PT" both get our pt-BR.
    assert_eq!(negotiate(Some("pt"), "en"), "pt-BR");
    assert_eq!(negotiate(Some("pt-PT"), "en"), "pt-BR");
    assert_eq!(negotiate(Some("en-US"), "pt-BR"), "en");
}

#[test]
fn test_negotiate_falls_back_to_the_default() {
    assert_eq!(negotiate(Some("fr"), "en"), "en");
    assert_eq!(negotiate(Some("fr"), "pt-BR"), "pt-BR");
    assert_eq!(negotiate(None, "en"), "en");
    assert_eq!(negotiate(Some(""), "en"), "en");
    assert_eq!(negotiate(Some("*"), "pt-BR"), "pt-BR");
}

#[test]
fn test_negotiate_follows_q_values() {
    // The browser prefers French, then Portuguese: we have no French.
    assert_eq!(
        negotiate(Some("fr-FR, fr;q=0.9, pt;q=0.8, en;q=0.7"), "en"),
        "pt-BR"
    );
    // Written order doesn't matter, the weight does.
    assert_eq!(negotiate(Some("en;q=0.5, pt-BR;q=0.9"), "en"), "pt-BR");
    // q=0 means "never this one".
    assert_eq!(negotiate(Some("pt-BR;q=0, en;q=0.1"), "pt-BR"), "en");
}

#[test]
fn test_default_locale_from_env_value() {
    assert_eq!(default_locale_from(None), "en");
    assert_eq!(default_locale_from(Some(" ".to_string())), "en");
    assert_eq!(default_locale_from(Some("pt-BR".to_string())), "pt-BR");
    assert_eq!(default_locale_from(Some(" pt ".to_string())), "pt-BR");
    // Unknown language: falls back to "en" (with a warning), never panics mid-request.
    assert_eq!(default_locale_from(Some("fr".to_string())), "en");
}

#[test]
fn test_hello_world_in_each_language() {
    for (locale, expected) in [("en", "Hello, world!"), ("pt-BR", "Olá, mundo!")] {
        let mut args = FluentArgs::new();
        args.set("name", tr(locale, "world", None));
        assert_eq!(tr(locale, "hello", Some(&args)), expected);
    }
}

// --- I18N-2: values and plurals inside messages ---

#[test]
fn test_name_inside_the_message_without_invisible_marks() {
    let mut args = FluentArgs::new();
    args.set("name", "Nick");
    // Exactly these bytes: no U+2068/U+2069 around "Nick".
    assert_eq!(tr("pt-BR", "hello", Some(&args)), "Olá, Nick!");
}

#[test]
fn test_cards_plural_forms() {
    let cases = [
        ("en", 1, "You have 1 card"),
        ("en", 3, "You have 3 cards"),
        ("pt-BR", 1, "Você tem 1 carta"),
        ("pt-BR", 3, "Você tem 3 cartas"),
    ];
    for (locale, count, expected) in cases {
        let mut args = FluentArgs::new();
        args.set("count", count);
        assert_eq!(
            tr(locale, "cards", Some(&args)),
            expected,
            "{locale}, {count}"
        );
    }
}

#[test]
fn test_unknown_key_comes_back_as_the_key() {
    assert_eq!(tr("pt-BR", "no-such-key", None), "no-such-key");
}

// --- I18N-3: every language has every key ---

fn message_keys(tag: &str, source: &str) -> BTreeSet<String> {
    let resource = fluent_syntax::parser::parse(source)
        .unwrap_or_else(|(_, errors)| panic!("i18n/{tag}.ftl has errors: {errors:?}"));
    resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            Entry::Message(message) => Some(message.id.name.to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn test_every_language_has_every_key() {
    let (_, fallback_source) = SOURCES
        .iter()
        .find(|(tag, _)| *tag == FALLBACK_LOCALE)
        .expect("the fallback language must be in SOURCES");
    let expected = message_keys(FALLBACK_LOCALE, fallback_source);

    let mut problems = vec![];
    for (tag, source) in SOURCES {
        let keys = message_keys(tag, source);
        for missing in expected.difference(&keys) {
            problems.push(format!("{tag} is missing: {missing}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
