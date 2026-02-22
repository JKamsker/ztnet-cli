# ZTNet CLI (Rust) — Implementation Checklist

Source spec: `docs/Tasks/Initial.md`

- [x] Create implementation checklist
- [x] Scaffold Rust CLI crate + Clap command tree (incl. `completion`)
- [x] Implement config file + profiles + precedence (flags > env > config > defaults)
- [x] Implement HTTP client (auth header, retries/backoff, timeout, dry-run, exit codes)
- [x] Implement output formats + logging flags (`--json`, `--output`, `--quiet`, `-v`, `--no-color`)
- [x] Implement `auth` and `config` commands
- [x] Implement `user create`
- [x] Implement `org` commands
- [x] Implement `network` commands
- [x] Implement `network member` commands + `member` alias
- [x] Implement `stats`, `planet`, and `export hosts`
- [x] Implement `api` escape hatch and `trpc` experimental commands
- [x] Refactor: split `src/app.rs` into modules (<500 lines/file)
- [x] Refactor: split `src/cli.rs` into modules (<500 lines/file)
- [x] Add Docker-based local ZTNet test harness + run CLI smoke test

## Robust host config + validation

- [x] Create task checklist
- [x] Support base URLs with path prefixes (`https://host/prefix`)
- [x] Add host normalization helpers (smart scheme, trim, trailing slash)
- [x] Add `ztnet config set host <URL>` alias + `config set --no-validate`
- [x] Validate host as ZTNet instance on set (try `/api` variants)
- [x] Validate token by default in `auth set-token` (`--no-validate` bypass)
- [x] Runtime host auto-fix (toggle `/api`) + big warning banner
- [x] Keep `host_defaults` consistent when host set/unset
- [x] Add unit tests for host normalization/candidates
- [x] Update docs (README + `docs/commands.md` + `docs/configuration.md`)
