// The app's shared state: what every route handler can reach (database pools, caches...).
// Packs only ADD lines here (`merge=union` in .gitattributes): one field in AppState and
// one line in build_app_state(). Write full paths (no `use` lines) so each line stands alone.

// Default exists only in tests: each pack makes its field default to an empty mock, so a
// unit test fills just the field it needs: `AppState { db_pool: ..., ..Default::default() }`.
#[derive(Clone)]
#[cfg_attr(test, derive(Default))]
pub struct AppState {
    pub telegram: Option<std::sync::Arc<crate::engine::telegram_engine::TelegramApi>>,
    // One field per pack, e.g. `pub db_pool: std::sync::Arc<crate::engine::db_engine::DbPool>,`
}

// Called once at startup: each pack connects to its service here.
pub async fn build_app_state() -> AppState {
    AppState {
        telegram: crate::engine::telegram_engine::TelegramApi::from_env(
            crate::handlers::telegram_handler::POLL_TIMEOUT_SECS,
        ),
        // One line per pack, e.g. `db_pool: crate::database::connection::start().await,`
    }
}
