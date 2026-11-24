// The binary only starts the app. The modules live in the library (src/lib.rs): declaring
// them here too compiled everything, and ran every unit test, twice.
use excelsior::utils::main_utils::service_starter;

#[tokio::main]
async fn main() {
    service_starter().await;
}
