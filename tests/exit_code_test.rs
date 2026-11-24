// The background-tasks pack's exit-code test, in its own file so the pack never edits the
// shared integration_test.rs (see "Writing a pack" in the README).
// Runs the real binary: when a worker fails, main() must exit with code 1, so Docker (or
// anything watching the process) sees a failure. This was lost once in a merge without
// any test noticing.
use std::process::Command;
use std::time::Duration;

#[test]
fn test_failing_worker_exits_with_code_1() {
    // The demo heartbeat fails after its first beat (10 seconds). Port 0: the OS picks a
    // free one, so this never clashes with another test or a running app.
    let mut child = Command::new(env!("CARGO_BIN_EXE_excelsior"))
        .env("MS_PORT", "0")
        .env("DEMO_HEARTBEAT_FAIL_AFTER", "1")
        .spawn()
        .expect("failed to start the app");

    // Give it at most 30s, then stop it, so a broken app never hangs the test run.
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            child.kill().ok();
            panic!("the app didn't stop after its worker failed");
        }
        std::thread::sleep(Duration::from_millis(200));
    };

    assert_eq!(
        status.code(),
        Some(1),
        "a failed worker must give exit code 1"
    );
}
