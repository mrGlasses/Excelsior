// Background workers: long-running loops that live beside the HTTP server.
//
// How to use (see service_starter):
//   let mut workers = Workers::new(shutdown_token.clone(), timeout);
//   workers.spawn("heartbeat", heartbeat_loop);
//   let any_failed = workers.run_until_shutdown().await;
//
// A worker is an async fn that receives a CancellationToken and returns anyhow::Result<()>.
// It must return soon after the token is cancelled (that's the "please stop" signal).
// If any worker fails (returns Err or panics), the token is cancelled for everyone,
// so the whole app shuts down and Docker can restart it.
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tokio::task::{Id, JoinError, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

// Used when WORKER_SHUTDOWN_TIMEOUT_SECS is not set. Under Docker's 10-second
// grace period, so we finish before Docker kills the container.
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 8;

pub struct Workers {
    tasks: JoinSet<anyhow::Result<()>>,
    // Which name belongs to which running task, for the logs.
    names: HashMap<Id, &'static str>,
    shutdown: CancellationToken,
    shutdown_timeout: Duration,
}

impl Workers {
    pub fn new(shutdown: CancellationToken, shutdown_timeout: Duration) -> Self {
        Workers {
            tasks: JoinSet::new(),
            names: HashMap::new(),
            shutdown,
            shutdown_timeout,
        }
    }

    // Starts a worker right away. Each worker gets its own copy of the token.
    pub fn spawn<F, Fut>(&mut self, name: &'static str, worker: F)
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let handle = self.tasks.spawn(worker(self.shutdown.clone()));
        self.names.insert(handle.id(), name);
        info!("worker {} started", name);
    }

    // Waits until the app is asked to stop (token cancelled) or a worker fails,
    // then stops every worker. Returns true if any worker failed.
    pub async fn run_until_shutdown(mut self) -> bool {
        let mut any_failed = false;

        // 1. Running: wait for the stop signal, watching for workers that end early.
        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => break,
                finished = self.tasks.join_next_with_id() => match finished {
                    // No workers left: nothing to watch, just wait for the stop signal.
                    None => {
                        self.shutdown.cancelled().await;
                        break;
                    }
                    Some(result) => {
                        if self.log_finished(result, "finished") {
                            any_failed = true;
                            // Tell everyone else (and the HTTP server) to stop.
                            self.shutdown.cancel();
                            break;
                        }
                    }
                },
            }
        }

        // 2. Stopping: every worker has seen the cancelled token, wait for them to return.
        let all_stopped = tokio::time::timeout(self.shutdown_timeout, async {
            let mut failed_while_stopping = false;
            while let Some(result) = self.tasks.join_next_with_id().await {
                if self.log_finished(result, "stopped") {
                    failed_while_stopping = true;
                }
            }
            failed_while_stopping
        })
        .await;

        match all_stopped {
            Ok(failed_while_stopping) => any_failed || failed_while_stopping,
            Err(_) => {
                // Someone ignored the stop signal: abandon it and carry on shutting down.
                for name in self.names.values() {
                    warn!(
                        "worker {} did not stop within {}s",
                        name,
                        self.shutdown_timeout.as_secs()
                    );
                }
                self.tasks.abort_all();
                any_failed
            }
        }
    }

    // Logs how a worker ended. Returns true if it failed.
    // `ok_word` is "finished" while running, "stopped" during shutdown.
    fn log_finished(
        &mut self,
        result: Result<(Id, anyhow::Result<()>), JoinError>,
        ok_word: &str,
    ) -> bool {
        match result {
            Ok((id, Ok(()))) => {
                info!("worker {} {}", self.take_name(id), ok_word);
                false
            }
            Ok((id, Err(err))) => {
                error!("worker {} failed: {}", self.take_name(id), err);
                true
            }
            // A panic counts as a failure, never as silence.
            Err(join_error) => {
                let name = self.take_name(join_error.id());
                if join_error.is_panic() {
                    error!(
                        "worker {} failed: panicked: {}",
                        name,
                        panic_message(join_error)
                    );
                } else {
                    error!("worker {} failed: {}", name, join_error);
                }
                true
            }
        }
    }

    fn take_name(&mut self, id: Id) -> &'static str {
        self.names.remove(&id).unwrap_or("unknown")
    }
}

// Shutdown timeout from the WORKER_SHUTDOWN_TIMEOUT_SECS value; unset means the default.
pub fn shutdown_timeout_from(value: Option<String>) -> Duration {
    match value {
        None => Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS),
        Some(text) => match text.trim().parse::<u64>() {
            Ok(secs) => Duration::from_secs(secs),
            Err(_) => panic!("WORKER_SHUTDOWN_TIMEOUT_SECS must be a number of seconds."),
        },
    }
}

// panic!("text") and panic!("{}", x) carry a &str or a String; anything else is rare.
fn panic_message(join_error: JoinError) -> String {
    let payload = join_error.into_panic();
    if let Some(text) = payload.downcast_ref::<&str>() {
        text.to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "unknown panic".to_string()
    }
}
