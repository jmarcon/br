# br — BrowserRouter

`br` is a cross-platform link/protocol router for Windows, macOS, and Linux.
Set it as your default browser and it will route each `http`/`https` link to
the right browser and profile based on rules you define — with optional URL
filters (tracking-parameter stripping, HTTP→HTTPS upgrade) and a fast picker
UI when no rule matches.

## Installation

### Windows
Download `br-windows-x86_64.msi` from the [latest release](../../releases/latest)
and run it. This installs `br`, `br-daemon`, and `br-settings` to your user
profile and adds a Start Menu shortcut for **BrowserRouter Settings**.

### macOS
Download `br-macos-x86_64.dmg` from the [latest release](../../releases/latest),
open it, and run `install.sh` (it copies the binaries to `/usr/local/bin`).

### Linux (Debian/Ubuntu)
Download `browserrouter-amd64.deb` from the [latest release](../../releases/latest)
and install it:

```sh
sudo dpkg -i browserrouter-amd64.deb
```

### From source
```sh
cargo build --workspace --release
```
Binaries are produced at `target/release/{br,br-daemon,br-settings}`.

## Getting started

1. Run `br-settings` to open the settings UI. On first run it walks you
   through onboarding:
   - **Discover browsers** — finds installed browsers and profiles.
   - **Set br as your default browser** — registers `br` as the system
     `http`/`https` handler (may require a manual confirmation step
     depending on your OS).
   - **Finish setup** — saves your configuration.
2. Open links as usual. `br` intercepts them, applies your rules and
   filters, and either launches the matching browser directly or shows a
   picker if no rule matches (or the rule is set to "ask").

## Configuration

Configuration lives at `<config-dir>/br/config.toml`:

- Windows: `%APPDATA%\br\config.toml`
- macOS: `~/Library/Application Support/br/config.toml`
- Linux: `~/.config/br/config.toml`

It contains:

- `[general]` — theme, language (`en`/`pt-BR`), picker position/timeout,
  start-on-login.
- `[[browsers]]` — discovered/configured browsers and profiles.
- `[[filters]]` — URL filters (strip tracking params, upgrade HTTP to
  HTTPS), with per-domain exceptions.
- `[[rules]]` — ordered rules matching on URL pattern and/or source
  application, routing to a browser, asking the user, or blocking the URL.

Use `br-settings` to edit configuration through the UI, or edit
`config.toml` directly.

## CLI usage

```sh
br open <url>              # route a URL (used internally as the default handler)
br doctor                   # diagnose configuration and environment
br config show               # print the resolved configuration
br config validate          # validate the configuration file
br rules list                 # list configured rules
br rules test <url>          # show which rule would handle a URL
br browsers list             # list detected browsers/profiles
br settings                   # open the settings UI
br daemon-status              # check whether br-daemon is running
br register                   # register br as the default http/https handler
br unregister                 # remove br as the default handler
```

Pass `--json` to most commands for machine-readable output, and `--config
<path>` to use a config file other than the default location.

## Updating

`br doctor` reports the running version. New releases are published on the
[releases page](../../releases) — download and reinstall the package for
your platform to update. Automatic update checks are not yet implemented.

## Localization

The settings and picker UIs are available in English (`en`) and
Brazilian Portuguese (`pt-BR`); set `language` under `[general]` in
`config.toml`.

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

See [PRD.md](PRD.md) for the full product specification and roadmap.
