# Underdogs Cup Helper Rust

[![Travis CI build status](https://travis-ci.com/IceDynamix/uc_helper_rust.svg?branch=main)](https://travis-ci.com/github/IceDynamix/uc_helper_rust) [![LoC Count](https://tokei.rs/b1/github/IceDynamix/uc_helper_rust)](https://github.com/XAMPPRocky/tokei)

Second rust rewrite of the Python
[Underdogs Cup Helper](https://github.com/IceDynamix/underdogs_cup_helper).

A Discord bot made to support the Underdogs Cup project, which is a lower-ranked
[Tetr.io] tournament held around every 2 months.

Contains functionality to help with tournament management, but also to provide utility for players who are looking to
view their stats or find their opponent.

Uses [MongoDB] as database backend and [Serenity] for the Discord bot side of things.

## Running

Following environment variables need to be set:

```ini
DATABASE_URL=<MongoDB database URL>
DISCORD_TOKEN=<Discord bot token>
RUST_LOG=<error/warn/info/debug/trace>
```

Make sure Rust is installed and set to stable release. Start the Discord bot with `cargo run`.

## Contributing

Not that I expect anyone to contribute to this, but just in case:

- Use [Clippy]
- Add documentation when working in `database` and `tetrio` modules

Open local documentation with `cargo docs --open`

That's about it thanks

[Tetr.io]: https://tetr.io

[MongoDB]: https://github.com/mongodb/mongo-rust-driver

[Serenity]: https://github.com/serenity-rs/serenity

[Clippy]: https://github.com/rust-lang/rust-clippy
