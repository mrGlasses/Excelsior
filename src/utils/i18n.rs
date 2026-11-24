// i18n (EX0035): text looked up by key, in the caller's language, from Fluent files in i18n/.
//
// The .ftl files are compiled into the binary with include_str! (nothing to copy into
// Docker, and Cargo rebuilds when one changes). A new language = a new i18n/<tag>.ftl and
// one line in SOURCES; the I18N-3 test then makes sure it has every key.
use axum::extract::FromRequestParts;
use axum::http::header::ACCEPT_LANGUAGE;
use axum::http::request::Parts;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use std::convert::Infallible;
use std::sync::LazyLock;
use tracing::warn;
use unic_langid::LanguageIdentifier;

// Every supported language: its tag and its messages.
pub const SOURCES: &[(&str, &str)] = &[
    ("en", include_str!("../../i18n/en.ftl")),
    ("pt-BR", include_str!("../../i18n/pt-BR.ftl")),
];

// The last resort: the language every other file is checked against.
pub const FALLBACK_LOCALE: &str = "en";

struct Language {
    tag: &'static str,
    bundle: FluentBundle<FluentResource>,
}

static LANGUAGES: LazyLock<Vec<Language>> = LazyLock::new(|| {
    SOURCES
        .iter()
        .map(|(tag, source)| {
            let id: LanguageIdentifier = tag
                .parse()
                .unwrap_or_else(|_| panic!("invalid language tag in SOURCES: {tag}"));
            let resource = FluentResource::try_new(source.to_string())
                .unwrap_or_else(|(_, errors)| panic!("i18n/{tag}.ftl has errors: {errors:?}"));
            let mut bundle = FluentBundle::new_concurrent(vec![id]);
            // Without this, Fluent wraps every inserted value in invisible direction marks
            // (U+2068/U+2069): "Olá, Nick!" would look right and still not be equal.
            bundle.set_use_isolating(false);
            bundle
                .add_resource(resource)
                .unwrap_or_else(|errors| panic!("i18n/{tag}.ftl repeats a key: {errors:?}"));
            Language { tag, bundle }
        })
        .collect()
});

static DEFAULT_LOCALE: LazyLock<&'static str> =
    LazyLock::new(|| default_locale_from(std::env::var("DEFAULT_LOCALE").ok()));

// The language used when the caller asks for nothing we have (DEFAULT_LOCALE, "en" if unset).
pub fn default_locale() -> &'static str {
    *DEFAULT_LOCALE
}

// Receives the value instead of reading the variable, so tests don't touch the environment.
pub fn default_locale_from(value: Option<String>) -> &'static str {
    let Some(value) = value.filter(|v| !v.trim().is_empty()) else {
        return FALLBACK_LOCALE;
    };
    supported(value.trim()).unwrap_or_else(|| {
        warn!(
            "DEFAULT_LOCALE={} has no i18n file, using {}",
            value, FALLBACK_LOCALE
        );
        FALLBACK_LOCALE
    })
}

// The best supported language for an Accept-Language header, e.g. "fr;q=0.9, pt;q=0.8".
// Higher q first (same q: the order written); the first one we have wins.
pub fn negotiate(accept_language: Option<&str>, default: &'static str) -> &'static str {
    let Some(header) = accept_language else {
        return default;
    };
    let mut wanted: Vec<(f32, usize, &str)> = header
        .split(',')
        .enumerate()
        .filter_map(|(position, part)| {
            let mut pieces = part.split(';');
            let tag = pieces.next()?.trim();
            let q = pieces
                .find_map(|p| p.trim().strip_prefix("q="))
                .map(|q| q.trim().parse::<f32>().unwrap_or(0.0))
                .unwrap_or(1.0);
            // q=0 means "not this one".
            (!tag.is_empty() && q > 0.0).then_some((q, position, tag))
        })
        .collect();
    wanted.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));

    for (_, _, tag) in wanted {
        if tag == "*" {
            return default;
        }
        if let Some(found) = supported(tag) {
            return found;
        }
    }
    default
}

// The exact tag first ("pt-BR"), then the same language ("pt", "pt-PT" -> "pt-BR").
fn supported(tag: &str) -> Option<&'static str> {
    let tags = || SOURCES.iter().map(|(t, _)| *t);
    tags().find(|t| t.eq_ignore_ascii_case(tag)).or_else(|| {
        let language = tag.split(['-', '_']).next()?;
        tags().find(|t| {
            t.split('-')
                .next()
                .is_some_and(|l| l.eq_ignore_ascii_case(language))
        })
    })
}

// The text for `key` in `locale`. A key that language lacks comes from DEFAULT_LOCALE,
// then "en"; a key nobody has comes back as the key itself (and a warning).
pub fn tr(locale: &str, key: &str, args: Option<&FluentArgs>) -> String {
    for tag in [locale, default_locale(), FALLBACK_LOCALE] {
        let Some(language) = LANGUAGES.iter().find(|l| l.tag == tag) else {
            continue;
        };
        let Some(pattern) = language.bundle.get_message(key).and_then(|m| m.value()) else {
            continue;
        };
        let mut errors = vec![];
        let text = language.bundle.format_pattern(pattern, args, &mut errors);
        if !errors.is_empty() {
            warn!("i18n: {tag}/{key}: {errors:?}");
        }
        return text.into_owned();
    }
    warn!("i18n: no language has the key {key}");
    key.to_string()
}

// The caller's language, as a handler argument: `locale: Locale`, then `tr(locale.0, ...)`.
pub struct Locale(pub &'static str);

impl<S: Send + Sync> FromRequestParts<S> for Locale {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok());
        Ok(Locale(negotiate(header, default_locale())))
    }
}
