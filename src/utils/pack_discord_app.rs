// The app behind the Discord bot (EX0036). The bot hands every slash command and button
// click to what this function returns, and registers its commands.
//
// An app built on Excelsior replaces the ONE line below with its own DiscordApp, e.g.
// `std::sync::Arc::new(crate::adapters::discord::GameDiscordApp::new(state.clone()))`.
// The pack writes this file once and never changes it, so that edit never conflicts.
// (Not `merge=union`: two lines here would not compile.)
use crate::engine::discord_engine::DiscordApp;
use crate::state::AppState;
use std::sync::Arc;

pub fn discord_app(_state: &AppState) -> Arc<dyn DiscordApp> {
    Arc::new(crate::handlers::discord_handler::DemoDiscordApp)
}
