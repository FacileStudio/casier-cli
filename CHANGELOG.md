# Changelog

All notable changes to this project are documented here. The format is
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). While on
`0.x`, a breaking change bumps the minor.

Every entry below was reconstructed from git history on 2026-08-24, so they
record what shipped rather than what was written down at the time. The v0.1.0
entry also covers the CLI's life inside the `Casier` monorepo, since the repo
split on 2026-08-06 carried that history over.

## [Unreleased]

## [0.4.3] - 2026-11-11

### Added

- casier-cli is released under the [MIT License](LICENSE).

### Fixed

- Dependencies carry the patched quinn-proto, rustls, rustls-webpki, webpki-roots, h2, tokio and rustix releases.

## [0.4.2] - 2026-11-03

### Fixed

- The Homebrew formula's Linux URL names the `casier` tarball instead of an unset variable, so the formula push completes.

## [0.4.1] - 2026-11-03

### Fixed

- The release workflow updates the Homebrew formula again: `gh release download` names the repository, so it no longer needs a git checkout.

## [0.4.0] - 2026-11-03

### Changed

- `casier login` signs in with the OAuth device grant when the server offers SSO. It prints an
  approval link to open in any browser and needs no port on this machine, so it works over SSH.
  Nothing to type or paste. `--no-browser` only stops it from opening the link.
- The loopback redirect is behind `casier login --listener`. A server without
  `/auth/oidc/device/exchange` falls back to it on its own.
- A 401 from the exchange says to sign in once at the web app in a browser, since the exchange
  never creates an account.

### Fixed

- The clippy 1.98 lints in `projects`, `secrets` and `run`.

## [0.3.0] - 2026-11-03

### Changed

- `casier login` takes the server as an optional argument and defaults to
  `https://casier.facile.studio`. It never prompts for a URL.
- When the server offers SSO and a browser is available, login opens the browser and finishes
  on its own, with no "Login with SSO?" question. `--no-browser` prints the SSO URL, and
  without a browser the CLI asks for email and password.
- The loopback page matches the shared Facile sign-in page, and a callback with the wrong
  state no longer ends the login.

## [0.2.1] - 2026-10-27

### Fixed

- Handle Linux keyring `NoEntry` on `Entry::new` when session keyring is absent.

## [0.2.0] - 2026-09-01

### Added

- `casier keys` command group for API key management (`list`, `create`, `revoke`).
- Support for creating secret and public API keys with daily quotas and allowed origins.
- JSON output support across all `casier keys` subcommands via `--json`.

## [0.1.1] - 2026-08-13

### Changed

- Login drives porte's login-code flow. The CLI now exchanges a single-use code
  for a bearer instead of reading a token straight off the loopback callback,
  which is the contract the server had already moved to.
- `install.sh` delegates to the `facile` CLI and bootstraps it from
  `get.facile.studio`, so installing casier no longer means a second set of
  install steps to keep in sync.

### Removed

- `openssl-sys`. reqwest builds its TLS with rustls, which drops the system
  OpenSSL from the build.

## [0.1.0] - 2026-08-10

### Added

- First release under its own repo. Terminal client for Casier: read and write
  secrets over the REST API, inject them into a child process, and keep `.env`
  files and Dokploy compose environments in sync.
- SSO login against a deployed Casier server, with the session token persisted
  and `casier.facile.studio` as the default server.
- An offline secret cache, `check`, `diff` and a Dokploy `push`.
- Per-repo configuration read from `casier.yml`.
- Prebuilt binaries published on tag, and an `install.sh` that fetches them.

### Changed

- Spaces became projects across the API, the client and the CLI.
- Setting a secret asks for its value explicitly and refuses a valueless
  answer, rather than storing an empty string nobody meant.
- The project was renamed from Clef to Casier.

[Unreleased]: https://github.com/FacileStudio/casier-cli/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/FacileStudio/casier-cli/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/FacileStudio/casier-cli/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/FacileStudio/casier-cli/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/FacileStudio/casier-cli/releases/tag/v0.1.0
