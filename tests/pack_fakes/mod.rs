// The pack registry for test fakes. The tests that start the whole app (service_starter)
// must never talk to a real outside service, so each pack that needs one (a bot API, a
// gateway...) adds ONE `pub mod <pack>;` line and ONE line to the list below.
// mod.rs files are `merge=union` in .gitattributes: two packs adding lines merge cleanly.
use std::any::Any;

// One file per pack, next to this one.
pub mod discord;

// Starts every pack's fake and sets the env vars that point the app at it.
// Keep the returned value until the test ends: dropping it stops the fakes.
pub async fn start_pack_fakes() -> Vec<Box<dyn Any>> {
    vec![
        // One line per pack, ending with a comma.
        Box::new(discord::start_fake_discord_gateway().await),
    ]
}
