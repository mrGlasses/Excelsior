// The app's shared state: what every route handler can reach (database pools, caches...).
// Packs only ADD lines here (`merge=union` in .gitattributes): one field in AppState and
// one line in build_app_state(). Write full paths (no `use` lines) so each line stands alone.

// Default exists only in tests: each pack makes its field default to an empty mock, so a
// unit test fills just the field it needs: `AppState { db_pool: ..., ..Default::default() }`.
#[derive(Clone)]
#[cfg_attr(test, derive(Default))]
pub struct AppState {
    pub discord: Option<std::sync::Arc<crate::engine::discord_engine::DiscordSender>>,
    // One field per pack, e.g. `pub db_pool: std::sync::Arc<crate::engine::db_engine::DbPool>,`
}

// Called once at startup: each pack connects to its service here.
pub async fn build_app_state() -> AppState {
    AppState {
        discord: crate::engine::discord_engine::DiscordSender::from_env(),
        // One line per pack, e.g. `db_pool: crate::database::connection::start().await,`
    }
}
