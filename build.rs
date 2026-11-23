// sqlx::migrate! copies the migration files into the binary at compile time, but Cargo
// doesn't know about them: adding or editing a .sql file didn't trigger a rebuild, so a
// local `cargo run`/`cargo test` kept the OLD migrations (EX0034: a broken migration still
// "passed"). This tells Cargo to rebuild whenever that folder changes.
fn main() {
    println!("cargo:rerun-if-changed=src/database/migrations");
}
