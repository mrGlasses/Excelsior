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

### Environment variables

- `DATABASE_MAX_CONNECTIONS` — optional, Postgres connection pool size (default `5`).

### Database migrations

The SQL files in `src/database/migrations` (`0001_...sql`, `0002_...sql`, ...) are applied
automatically at startup by sqlx, and recorded in the `_sqlx_migrations` table. Never edit a
file that has already been applied (the app refuses to start if you do); add a new numbered file instead.

## Versioning

Excelsior's version (`version` in `Cargo.toml`) is only changed on `main`. Packs never edit it:
they get it when they're updated with `main` (`git merge main`), so every pack shows the version
of the base it's built on.

- **Major** (3.0.0): breaks the packs or projects built on Excelsior: the D.A.F.T. layout, where
  routes and workers are wired, the startup flow (`service_starter()`, `AppState`), a breaking
  Axum/Tokio/sqlx major upgrade, or a new Rust edition/minimum version. Every pack is updated and retested.
- **Minor** (2.2.0): something new in the base that doesn't break existing code (a middleware,
  a utility, an optional env var with a default).
- **Patch** (2.1.2): fixes only: security/dependency bumps, bug and test fixes, docs.

Adding or updating a pack does not change Excelsior's version. Each pack has its own file at the
root of the repo, `pack-<name>.toml`, that only that pack edits (so merging packs never conflicts on it):

```toml
name = "outbox-postgresql"
version = "1.0.0"     # the pack's own version
excelsior = "2.1"     # the Excelsior major.minor it was built and tested on
ticket = "EX0028"
requires = ["postgresql-sqlx-keep-alive", "background-tasks"]
```

`ls pack-*` lists the packs in a project. Every name in `requires` needs its own `pack-*.toml` too.
After a minor or major Excelsior version, each pack is updated with `main`, retested, and its
`excelsior` value changed.

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

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/mrGlasses/Excelsior/blob/main/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Excelsior by you, shall be licensed as MIT, without any additional
terms or conditions.

### Contributors

- Nick G. (Main Dev)