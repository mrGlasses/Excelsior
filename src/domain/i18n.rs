// Query parameters of the i18n demo routes (EX0035).
use serde::Deserialize;

// GET /hello?name=Nick (name is optional: "world", translated, when missing)
#[derive(Debug, Deserialize)]
pub struct HelloParams {
    pub name: Option<String>,
}

// GET /cards?count=3
#[derive(Debug, Deserialize)]
pub struct CardsParams {
    pub count: u32,
}
