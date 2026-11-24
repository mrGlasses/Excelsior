// The binary only starts the app. The modules live in the library (src/lib.rs): declaring
// them here too compiled everything, and ran every unit test, twice.
use excelsior::utils::main_utils::service_starter;

#[tokio::main]
async fn main() {
    let clean_shutdown = service_starter().await;
    if !clean_shutdown {
        // A worker failed: exit with 1 so Docker's "restart: always" brings the app back.
        std::process::exit(1);
    }
}
