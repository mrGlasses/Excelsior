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

- `WORKER_SHUTDOWN_TIMEOUT_SECS` — optional, seconds each background worker gets to stop on shutdown before it's abandoned (default `8`, under Docker's 10-second grace period).
- `DEMO_HEARTBEAT_FAIL_AFTER` — optional, demo only: makes the `heartbeat` worker fail after that many beats, so the app exits with code 1.
- `DEMO_STUCK_WORKER` — optional, demo only: `true` starts a worker that ignores the stop signal, to see the shutdown timeout.
- `TELEGRAM_BOT_TOKEN` — **required**, the bot token from @BotFather; the app exits at start without it. Don't put a real token in `.env` (it's tracked in git): set it in your shell or IDE run configuration.
- `TELEGRAM_API_URL` — optional, Telegram Bot API base URL (default `https://api.telegram.org`); tests point it at a fake server.

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

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/mrGlasses/Excelsior/blob/main/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Excelsior by you, shall be licensed as MIT, without any additional
terms or conditions.

### Contributors

- Nick G. (Main Dev)