// i18n demo routes (EX0035): the same handler answers in the caller's language.
use crate::domain::i18n::{CardsParams, HelloParams};
use crate::utils::i18n::{Locale, tr};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use fluent_bundle::FluentArgs;

// GET /hello and /hello?name=Nick -> "Hello, world!", "Olá, Nick!"...
pub async fn hello(locale: Locale, Query(params): Query<HelloParams>) -> Response {
    let name = params.name.unwrap_or_else(|| tr(locale.0, "world", None));
    let mut args = FluentArgs::new();
    args.set("name", name);
    (StatusCode::OK, tr(locale.0, "hello", Some(&args))).into_response()
}

// GET /cards?count=3 -> "You have 3 cards", "Você tem 1 carta"...
pub async fn cards(locale: Locale, Query(params): Query<CardsParams>) -> Response {
    let mut args = FluentArgs::new();
    // A number, not text: Fluent picks the plural form from it.
    args.set("count", params.count);
    (StatusCode::OK, tr(locale.0, "cards", Some(&args))).into_response()
}
