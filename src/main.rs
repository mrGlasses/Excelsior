use crate::utils::main_utils::service_starter;

mod database;
mod domain;
mod engine;
mod handlers;
mod routes;
mod state;
mod utils;

#[tokio::main]
async fn main() {
    service_starter().await;
}
