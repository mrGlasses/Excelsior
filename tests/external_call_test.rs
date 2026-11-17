// The http-external-call pack's own integration test, in its own file so the pack never
// edits the shared integration_test.rs (see "Writing a pack" in the README).
// It calls the real demo service in EXTERNAL_SERVICE_URL, so it needs internet access.
use excelsior::routes;
use excelsior::state::build_app_state;
use tokio::net::TcpListener;

// Starts the whole app (every pack's state) on a free port.
async fn spawn_app() -> String {
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = routes::create_routes(build_app_state().await);
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn test_its_a_rainy_day_route() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/its-a-rainy-day", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    let body = response.text().await.unwrap();
    assert!(body.contains("Ladaradiradadada! !!"));
}
