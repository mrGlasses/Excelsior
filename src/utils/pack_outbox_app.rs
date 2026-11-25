// The app behind the outbox worker (EX0038). The worker hands every committed event to
// what this function returns.
//
// An app built on Excelsior replaces the ONE line below with its own OutboxApp, e.g.
// `std::sync::Arc::new(crate::workers::dispatcher::GameOutboxApp::new(state.clone()))`.
// The pack writes this file once and never changes it, so that edit never conflicts.
// (Not `merge=union`: two lines here would not compile.)
use crate::engine::outbox_engine::OutboxApp;
use crate::state::AppState;
use std::sync::Arc;

pub fn outbox_app(_state: &AppState) -> Arc<dyn OutboxApp> {
    Arc::new(crate::handlers::outbox_handler::DemoOutboxApp)
}
