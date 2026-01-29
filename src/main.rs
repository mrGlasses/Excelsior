use excelsior::utils::main_utils::service_starter;

mod handlers;
mod routes;
mod utils;

#[tokio::main]
async fn main() {
    service_starter().await;
}
