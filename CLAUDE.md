# casier-cli

Terminal client for [Casier](https://github.com/FacileStudio/Casier), the Facile self-hosted
secrets manager. Reads and writes secrets over the Casier REST API, injects them into a child
process, and keeps `.env` files and Dokploy compose environments in sync.

Split out of the `Casier` monorepo (`cli/`) on 2026-08-06, following the suite convention that
every CLI is its own repo. The server lives in `FacileStudio/Casier`; changes to the API surface
have to land in both.

## Tech stack

- Language: Rust (edition 2021), async on Tokio
- HTTP: reqwest (JSON)
- CLI parsing: clap (derive)
- Config: the machine config is TOML (`toml`); the per-repo `casier.yml` is YAML (`serde_norway`), with `.casier.toml` still read
- Token storage: `keyring` (OS keychain)

## Commands

```sh
cargo build              # debug build
cargo build --release    # optimized (LTO + strip)
cargo test               # unit tests — api, cache, check, envfile, loopback
cargo fmt --check        # formatting gate
cargo run -- projects    # run a subcommand locally
```

## Project structure

```
src/
  main.rs        clap definition and subcommand dispatch
  api.rs         Casier REST client, Secret / RevealedSecret / MissingValues
  auth.rs        Keychain token storage, CASIER_TOKEN override
  config.rs      <config_dir>/casier/config.toml, casier.yml, project/env resolution
  cache.rs       Offline secret cache backing `run --offline`
  envfile.rs     .env parsing and serialization
  loopback.rs    Ephemeral 127.0.0.1 listener for the SSO login round trip
  commands/      login, logout, init, projects, secrets, run, check, diff, sync, push
integrations/
  SKILL.md       AI agent skill, registered by install.sh
```

## Load-bearing details

**`keyring` must keep its per-target platform features** in `Cargo.toml` (`apple-native`,
`windows-native`, `sync-secret-service`). With none of them enabled the crate silently falls back
to an in-memory mock store: login prints "Logged in as …", writes the config, and the token
disappears when the process exits, so every later command reports "Not logged in". `store_token`
reads the token back after writing to make that failure loud if it ever regresses.

**The API is at `/api` in production and at the root in development.** Traefik serves Casier's API
under `/api` and strips the prefix before the Go app. `login` therefore probes `<url>/auth/config`
and falls back to `<url>/api/auth/config`, saving whichever answers — so both
`https://casier.facile.studio` and `.../api` work as a `--server` value.

**SSO login uses a loopback listener, not a device code.** The CLI calls
`/auth/oidc?cli_port=<port>&cli_state=<nonce>`; after the OIDC round trip the API mints a
*separate* session token and redirects to `http://127.0.0.1:<port>/callback`. The port and nonce
are validated server-side and the redirect host is hard-coded, so the parameters cannot be turned
into an open redirect. The CLI checks the nonce to reject token injection from other local
processes.

**Reading a value is a different request from listing keys.** `list_secrets` returns metadata;
`reveal_secrets` / `reveal_secret` hit `?reveal=true` and `/secrets/{key}/reveal`, which the server
audits separately. `RevealedSecret` carries a `String` where `Secret` carries an `Option<String>`,
so "did the server actually send values?" is answered once at the boundary. A valueless answer to
a revealing read means this binary is older than the server and raises `MissingValues` —
`run` deliberately does **not** fall back to its offline cache for that error, because the server
was reachable and a stale cache would hide the bug rather than surface it.

**`CASIER_TOKEN` beats the keychain**, which is the only way to authenticate in CI and containers.
`CASIER_SERVER_URL` likewise beats the saved config.

## Conventions

- No inline comments; doc comments (`///`) carry the reasoning that is not obvious from the code
- Remove dead code rather than allowing it
- Non-trivial logic gets one runnable test — no fixtures, no frameworks
- Commit style: capitalized imperative sentence, body explains why
