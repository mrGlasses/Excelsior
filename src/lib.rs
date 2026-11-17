// Unit tests write `AppState { one_field: ..., ..Default::default() }` (see state.rs). With
// a single pack installed the `..Default::default()` fills nothing, which clippy flags.
#![cfg_attr(test, allow(clippy::needless_update))]

pub mod database;
pub mod domain;
pub mod engine;
pub mod handlers;
pub mod routes;
pub mod state;
pub mod utils;
// Packs add their `mod` lines above this one (see "Writing a pack" in the README).
