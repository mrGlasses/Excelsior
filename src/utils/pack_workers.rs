// The pack registry for background workers. Each pack that runs a worker adds ONE line to
// the list below, e.g. `crate::handlers::discord_handler::start(workers, state),`.
// This file is `merge=union` in .gitattributes: when two packs add their line at the same
// spot, git keeps both. Write full paths (no `use` lines) so a pack only adds that one line.
use crate::state::AppState;
use crate::utils::workers::Workers;

// Each start() reads its settings and spawns its worker(s). It returns false when the pack
// can't start (a missing token, for example): the app then exits with code 1.
// Every start() runs, so all the missing settings are logged at once. `state` is there for
// workers that need a pack's pool (the outbox worker needs the database, for example).
pub fn start_pack_workers(workers: &mut Workers, state: &AppState) -> bool {
    let started: Vec<bool> = vec![
        // One line per pack, ending with a comma.
        crate::handlers::worker_handler::start(workers, state),
        crate::handlers::outbox_handler::start(workers, state),
    ];
    started.into_iter().all(|ok| ok)
}
