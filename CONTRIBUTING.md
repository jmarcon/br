# Contributing

## Setup

- Install Rust from `rust-toolchain.toml`.
- Run `cargo build --workspace`.
- Run checks before opening a PR:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

On Windows without MSVC Build Tools, use:

```sh
cargo +stable-x86_64-pc-windows-gnu test --workspace
cargo +stable-x86_64-pc-windows-gnu clippy --workspace --all-targets -- -D warnings
```

## Commits

Use Conventional Commits:

- `feat: add browser discovery`
- `fix: preserve query params`
- `docs: update config examples`
- `ci: add release build`

## Pull Requests

- Keep changes focused.
- Add tests for routing, filters, config parsing, and platform parsers.
- Update `CHANGELOG.md` for user-visible changes.
- Ensure CI is green.

