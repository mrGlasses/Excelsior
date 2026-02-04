use crate::utils::main_utils::service_starter;

mod domain;
mod handlers;
mod routes;
mod utils;

#[tokio::main]
async fn main() {
    service_starter().await;
}
