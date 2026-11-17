// Engines: how the app talks to outside services (databases, caches, bot APIs...).
pub mod discord_engine;
// Packs add their `mod` lines above this one (see "Writing a pack" in the README).

#[cfg(test)]
mod tests;
