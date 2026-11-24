// i18n demo (EX0035): answers by the Accept-Language header.
use crate::handlers::i18n_handler::{cards, hello};
use crate::state::AppState;
use axum::{Router, routing::get};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/hello", get(hello)) // curl -H 'Accept-Language: pt-BR' 'localhost:3000/hello?name=Nick'
        .route("/cards", get(cards)) // curl -H 'Accept-Language: pt-BR' 'localhost:3000/cards?count=3'
}
