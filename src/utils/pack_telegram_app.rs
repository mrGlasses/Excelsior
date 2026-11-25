// The app behind the Telegram bot (EX0037). The bot hands every update (message, button
// tap) to what this function returns.
//
// An app built on Excelsior replaces the ONE line below with its own TelegramApp, e.g.
// `std::sync::Arc::new(crate::adapters::telegram::GameTelegramApp::new(state.clone()))`.
// The pack writes this file once and never changes it, so that edit never conflicts.
// (Not `merge=union`: two lines here would not compile.)
use crate::engine::telegram_engine::TelegramApp;
use crate::state::AppState;
use std::sync::Arc;

pub fn telegram_app(_state: &AppState) -> Arc<dyn TelegramApp> {
    Arc::new(crate::handlers::telegram_handler::DemoTelegramApp)
}
