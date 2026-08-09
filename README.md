# casier

Terminal client for [Casier](https://casier.facile.studio), the Facile self-hosted secrets and
environment variable manager.

Injects secrets straight into a process, keeps a `.env` in sync with a server, gates CI on missing
keys, and pushes an environment to Dokploy. Tokens live in the OS keychain, never in a dotfile.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/FacileStudio/casier-cli/main/install.sh | bash
```

Installs to `~/.local/bin`. Pass `--bin-dir <dir>` to change that, `--source` to build from
source, `--no-skill` to skip AI agent skill registration. Building from source needs `cargo`
and `git` on `PATH`.

```sh
cargo install --git https://github.com/FacileStudio/casier-cli.git --force
```

## Setup

```sh
casier login                                  # prompts for the server, opens a browser under SSO
casier login --server http://localhost:4000   # against a local API
casier init                                   # writes casier.yml so -p/-e become optional
```

`login` resolves the server from `--server`, then `CASIER_SERVER_URL`, then the saved config, then
a prompt defaulting to `https://casier.facile.studio/api`, and saves the winner to
`<config_dir>/casier/config.toml`. It probes both `<url>/auth/config` and `<url>/api/auth/config`,
so either form of the production URL works.

`casier.yml` sets the per-project defaults:

```yaml
project:
  slug: my-project
  environment: dev
```

`.casier.toml` is still read for repositories that have not been converted, and
`casier.yml` wins when both are present.

## Usage

```sh
casier projects
casier secrets list -p my-project -e prod            # values masked
casier secrets list -p my-project -e prod --show     # values revealed, and audited as such
casier secrets set -p my-project -e prod API_KEY sk-…
casier run -- bun dev                                # inject and run, nothing hits the disk
casier run --offline -- bun dev                      # last cached read, for a lost network
casier check .env                                    # exit 1 if the server lacks a key
casier diff -p my-project --from dev --to prod
casier sync push -p my-project -e dev -f .env
casier sync pull -p my-project -e dev -f .env
casier push dokploy <composeId>                      # needs DOKPLOY_URL + DOKPLOY_API_KEY
```

## CI

There is no keychain in CI. Set `CASIER_TOKEN` to a `casier_…` API token and skip `login`
entirely:

```sh
CASIER_TOKEN=casier_… casier check .env -p my-project -e prod
CASIER_TOKEN=casier_… casier run -p my-project -e prod -- ./deploy.sh
```

Scope the token to one project, one environment and read-only permissions — the server enforces
all three.

## AI agent integration

`install.sh` auto-registers casier as an AI agent skill for Claude Code and Codex, so assistants
can reach for it when you ask about secrets or environment variables.
