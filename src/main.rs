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
    let clean_shutdown = service_starter().await;
    if !clean_shutdown {
        // A worker failed: exit with 1 so Docker's "restart: always" brings the app back.
        std::process::exit(1);
    }
}
