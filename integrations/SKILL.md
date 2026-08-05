---
name: casier
description: >
  Facile secrets manager CLI. Use when the user asks to read, set, inject or
  sync environment variables and secrets, or mentions Casier.
---

# casier — Facile secrets manager

Binary: `casier`
Config: `<config_dir>/casier/config.toml` (server URL) + `.casier.toml` (per-project defaults)
Token: OS keychain, or `CASIER_TOKEN` in CI

## When to apply

Use when the user mentions secrets, environment variables, `.env` files, injecting config into a
command, or Casier.
Triggers: "secret", "env var", ".env", "environment variable", "API key", "casier", "rotate",
"inject secrets", "sync env"

## Commands

### Setup
```
casier login [--server <url>] [--no-browser]   Authenticate (opens a browser under SSO)
casier logout                                  Clear the stored token
casier init                                    Write .casier.toml in the current project
casier projects                                List projects you belong to
```

### Secrets
```
casier secrets list -p <project> -e <env> [--show]   List keys (values masked without --show)
casier secrets get -p <project> -e <env> <key>       Read one value
casier secrets set -p <project> -e <env> <key> <val> Write a value
casier secrets delete -p <project> -e <env> <key>    Delete a secret
```

### Running and checking
```
casier run [-p <project>] [-e <env>] [--offline] -- <command>
casier check [file] [-p <project>] [-e <env>]   Exit 1 if the .env has keys the server lacks
casier diff -p <project> --from <env> --to <env>
```

### Sync and deploy
```
casier sync push -p <project> -e <env> -f .env   Upload a .env
casier sync pull -p <project> -e <env> -f .env   Download to a .env
casier push dokploy <composeId> [-p <project>] [-e <env>]
```

## Rules
- `-p`/`-e` default to `.casier.toml`, then `dev` — omit them inside a configured project
- Prefer `casier run -- <cmd>` over writing a `.env`; it never touches the disk
- `--show` and `get` reveal plaintext and are audited server-side as such — do not use them to
  fill a variable you could have injected with `run`
- `--offline` reuses the last cached read; it is for a lost network, not for speed
- `push dokploy` needs `DOKPLOY_URL` and `DOKPLOY_API_KEY`, and overwrites the compose env
- In CI, set `CASIER_TOKEN` — there is no keychain there
- Run `casier <cmd> -h` for exact syntax when unsure
