# Excelsior

The Safe and Easy Rust Web Ready-To-Go Archetype.

[Guides](https://medium.com/@realmr_glasses/excelsior-the-safe-and-easy-rust-web-ready-to-go-archetype-184207895ce0)

## Overview

Excelsior is a functional and readable repository to easily build a web service/API with the Rust language, built
basically with Tokio, Axum, and Tower libraries. And uses D.A.F.T. for folder organization.

## Usage

Make sure you have a full installation of the Rust compiler equal to or above version 1.87 on your machine. Each branch
in the "exp-pack" folder is an extra feature/example to add to your project.

1. Clone this code
2. Merge the branches you'll need
3. Create your new app

Some IDEs allow you to use a repository as a base to avoid cloning, see with your IDE.

Before merging packs, run this once in your clone. It lets git merge `Cargo.lock` by itself
(see `.gitattributes`); without it, a `Cargo.lock` conflict is fixed by keeping either side
and running `cargo build`:

```
git config merge.cargo-lock.driver "git merge-file --ours %A %O %B"
```


### Environment variables

- `DATABASE_MAX_CONNECTIONS` — optional, Postgres connection pool size (default `5`).
- `WORKER_SHUTDOWN_TIMEOUT_SECS` — optional, seconds each background worker gets to stop on shutdown before it's abandoned (default `8`, under Docker's 10-second grace period).
- `DEMO_HEARTBEAT_FAIL_AFTER` — optional, demo only: makes the `heartbeat` worker fail after that many beats, so the app exits with code 1.
- `DEMO_STUCK_WORKER` — optional, demo only: `true` starts a worker that ignores the stop signal, to see the shutdown timeout.
- `OUTBOX_MAX_ATTEMPTS` — optional, how many times the outbox worker tries a failing event before setting it aside (default `10`, at most `100`).
- `OUTBOX_RETRY_BASE_SECS` — optional, seconds a failed event waits before its next try, doubled each time, at most an hour (default `5`; `0` = retry on the next pass).

### Outbox (PostgreSQL)

Events are written to the `outbox` table in the same transaction as the change they
describe (`insert_outbox_event(&mut tx, ...)`), and handled by the `outbox` worker
(`handlers/outbox_handler.rs`). Postgres wakes the worker on commit (`LISTEN/NOTIFY`),
it re-checks every 5 seconds anyway, retries failing events up to `OUTBOX_MAX_ATTEMPTS`,
and `FOR UPDATE SKIP LOCKED` lets several app copies share the work without doubles.
Demo: `curl -X POST localhost:3000/demo-events -H 'Content-Type: application/json' -d '{"msg":"hi"}'`.
Plugging in your app (EX0038): implement `OutboxApp` (`handle(&event)`: `Ok` = published,
`Err` = retried with backoff) and change the one line in `src/utils/pack_outbox_app.rs`.
Delivery is at least once, so a handler must cope with seeing an event twice.

### Database migrations

The SQL files in `src/database/migrations` (`0001_...sql`, `0002_...sql`, ...) are applied
automatically at startup by sqlx, and recorded in the `_sqlx_migrations` table. Never edit a
file that has already been applied (the app refuses to start if you do); add a new numbered file instead.

## Versioning

Excelsior's version (`version` in `Cargo.toml`) is only changed on `main`. Packs never edit it:
they get it when they're updated with `main` (`git merge main`), so every pack shows the version
of the base it's built on.

- **Major** (4.0.0): breaks the packs or projects built on Excelsior: the D.A.F.T. layout, where
  routes and workers are wired, the startup flow (`service_starter()`, `AppState`), a breaking
  Axum/Tokio/sqlx major upgrade, or a new Rust edition/minimum version. Every pack is updated and retested.
- **Minor** (3.1.0): something new in the base that doesn't break existing code (a middleware,
  a utility, an optional env var with a default).
- **Patch** (3.0.1): fixes only: security/dependency bumps, bug and test fixes, docs.

Adding or updating a pack does not change Excelsior's version. Each pack has its own file at the
root of the repo, `pack-<name>.toml`, that only that pack edits (so merging packs never conflicts on it):

```toml
name = "outbox-postgresql"
version = "1.0.0"     # the pack's own version
excelsior = "3.0"     # the Excelsior major.minor it was built and tested on
ticket = "EX0028"
requires = ["postgresql-sqlx-keep-alive", "background-tasks"]
```

`ls pack-*` lists the packs in a project. Every name in `requires` needs its own `pack-*.toml` too.
After a minor or major Excelsior version, each pack is updated with `main`, retested, and its
`excelsior` value changed.

### Writing a pack

Any set of packs must merge without conflicts. `.gitattributes` makes git keep both sides of
the files packs add lines to (`.env`, `.env.test`, `README.md`, `Cargo.toml`, `mod.rs` files and
the registries below). For that to work:

- Only **add** lines to shared files; never change, reorder or delete an existing line.
- `mod` lines go right **above the marker comment** in each `mod.rs` and `lib.rs`
  ("Packs add their `mod` lines above this one"), never in alphabetical order: lines added
  around the same existing line by three packs get duplicated by git. `rustfmt.toml` keeps
  `cargo fmt` from sorting them. The `engine/`, `database/` and `domain/tests/` modules
  already exist, so a pack never creates its own `mod.rs`.
- New `Cargo.toml` sections (`[profile.release]`, `[features]`...) go below the marker at the
  end of the file; dependency lines go in their `[dependencies]` / `[dev-dependencies]` lists.
- A dependency used by two packs goes in the code they both build on, never in both. The
  common ones (`anyhow`, `async-trait`, `serde`, `serde_json`, `mockall`, `httpmock`) are
  already in `main`.
- Never edit `main_utils.rs`, `simple_handler.rs` or the existing tests. A pack's code and
  tests go in its own files, and it plugs in through the registries, one line each:

| A pack that needs...            | adds one line to                                                    |
|---------------------------------|---------------------------------------------------------------------|
| shared state (a pool, a client) | `src/state.rs`: a field in `AppState` and its line in `build_app_state()` |
| routes                          | `src/routes/mod.rs`: `pub mod <pack>_routes;` and `.merge(<pack>_routes::routes())` |
| a tracing layer / a final flush | `src/utils/pack_tracing.rs`: `pack_tracing_layers()` / `pack_shutdown()` |
| a background worker             | `src/utils/pack_workers.rs` (from `background-tasks`)                |
| a fake service in tests         | `tests/pack_fakes/mod.rs` (from `background-tasks`)                  |

Write full paths in those lines (`crate::engine::db_engine::DbPool`), not `use` lines. In unit
tests, build the state with only your field: `AppState { db_pool: ..., ..Default::default() }`.
For that, your field's type needs a `#[cfg(test)] impl Default` (an empty mock is enough).

## Contributing

Thanks for your help in improving the project! If you're willing to contribute, you'll have to follow several rules:

1. Create an Issue or a Pull Request
2. If you create a Pull Request, ensure you have followed the code model
3. Avoid unintelligible naming (like var a, b, method procedure_x)
4. Avoid theoretical code and variant typing (T), unless it's ULTRA necessary
5. Write a code that could be read by a junior Rust dev
6. Create the unit and integrated tests as needed

If you can't follow these rules, create an Issue and drop your code there, and I'll make it work.
Excelsior was created to be the "ready-made cake batter" for an API, not the cake recipe with the ingredients.

Before the code is merged, it will be tested in [ExcelsiorFull]. I like to ensure that all code works perfectly and is
integrated with all the other parts. After that, the code will be tested in the merge with the main Excelsior project.

[ExcelsiorFull]: https://github.com/mrGlasses/ExcelsiorFull

## Related Projects

In addition to the crates in this repository, the Excelsior project also maintains
several other libraries, including:

* [`ExcelsiorFull`]: The Excelsior version for feature testing and global integration.
* [`ExcelsiorExamples`]: A collection of examples using the Excelsior project.

[`ExcelsiorFull`]: https://github.com/mrGlasses/ExcelsiorFull

[`ExcelsiorExamples`]: https://github.com/mrGlasses/ExcelsiorExamples

## Changelog

- 1.0.0: Initial release.
- 2.0.0: Improving the code coverage and upgrading the dependencies.
- 2.1.2: Security fixes (RUSTSEC-2026-0007, -0258, -0477, -0285, -0382, -0391, RUSTSEC-2025-0055), versioning rules and `pack-<name>.toml` files.
- 2.1.3: Packs merge together without conflicts (`.gitattributes`, `Cargo.lock` merge driver), tracing set up once in tests.
- 3.0.0: Pack registries for shared state (`AppState`), routes and tracing; `create_routes()` takes the state. Packs plug in with one line each.
- 3.0.1: The dependencies many packs share (`anyhow`, `async-trait`, `serde`, `serde_json`, `mockall`, `httpmock`) live in `main`; README notes on merging packs.
- 3.1.0: Marker comments for `mod` lines and `rustfmt.toml` (`reorder_modules = false`), empty `engine/`, `database/` and `domain/tests/` modules, `lib.rs`/`main.rs` merged as union, `.env` files end with a newline.
- 3.1.1: Adds `rustfmt.toml`, missing from the 3.1.0 commit.
- 3.1.2: Fix: the app's own logs were switched off when no pack adds a tracing layer (since 3.0.0).
- 3.1.3: Marker at the end of `Cargo.toml`: packs add new sections below it.
- 3.1.4: `EXTERNAL_SERVICE_URL` leaves `main`'s `.env` files: only `http-external-call` uses it, and it adds the line itself.
- 3.1.5: `main.rs` only starts the app and uses the library: everything was compiled, and every unit test run, twice.

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/mrGlasses/Excelsior/blob/main/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Excelsior by you, shall be licensed as MIT, without any additional
terms or conditions.

### Contributors

- Nick G. (Main Dev)