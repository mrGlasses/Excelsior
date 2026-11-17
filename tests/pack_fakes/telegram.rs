use std::time::Duration;

// A fake Telegram Bot API for the tests that start the whole app: service_starter()
// needs a bot token, and tests must never talk to the real Telegram.
// Keep the returned server alive for the whole test.
pub async fn start_fake_telegram() -> httpmock::MockServer {
    let server = httpmock::MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method("POST").path("/botTEST-TOKEN/getMe");
            then.status(200).json_body(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "first_name": "Test", "username": "TestBot"}
            }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method("POST").path("/botTEST-TOKEN/getUpdates");
            // A real long poll waits when nothing happens; the delay stops a busy loop.
            then.status(200)
                .delay(Duration::from_millis(200))
                .json_body(serde_json::json!({"ok": true, "result": []}));
        })
        .await;
    unsafe {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "TEST-TOKEN");
        std::env::set_var("TELEGRAM_API_URL", server.base_url());
    }
    server
}
