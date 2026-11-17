// The Discord pack's own startup test, in its own file so it never touches the
// shared integration_test.rs (see "Writing a pack" in the README).
use excelsior::utils::main_utils::service_starter;
use std::time::Duration;

mod pack_fakes;

#[tokio::test]
async fn test_service_starter_without_discord_token_exits() {
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
    // Every other pack gets its fake, so only the Discord token is missing.
    let _fakes = pack_fakes::start_pack_fakes().await;
    // Empty rather than removed: dotenv() inside service_starter never overwrites an
    // existing variable, so a real token in .env can't sneak back in.
    unsafe {
        std::env::set_var("DISCORD_TOKEN", "");
    }

    let clean_shutdown = tokio::time::timeout(Duration::from_secs(5), service_starter())
        .await
        .expect("service_starter should return at once without a token");

    // false = main() exits with code 1
    assert!(!clean_shutdown);
}
