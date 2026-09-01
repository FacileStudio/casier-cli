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

[Unreleased]: https://github.com/FacileStudio/casier-cli/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/FacileStudio/casier-cli/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/FacileStudio/casier-cli/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/FacileStudio/casier-cli/releases/tag/v0.1.0
