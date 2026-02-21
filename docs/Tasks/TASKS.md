# ZTNet CLI (Rust) — Implementation Checklist

Source spec: `docs/Tasks/Initial.md`

- [x] Create implementation checklist
- [x] Scaffold Rust CLI crate + Clap command tree (incl. `completion`)
- [x] Implement config file + profiles + precedence (flags > env > config > defaults)
- [x] Implement HTTP client (auth header, retries/backoff, timeout, dry-run, exit codes)
- [x] Implement output formats + logging flags (`--json`, `--output`, `--quiet`, `-v`, `--no-color`)
- [ ] Implement `auth` and `config` commands
- [ ] Implement `user create`
- [ ] Implement `org` commands
- [ ] Implement `network` commands
- [ ] Implement `network member` commands + `member` alias
- [ ] Implement `stats`, `planet`, and `export hosts`
- [ ] Implement `api` escape hatch and `trpc` experimental commands
- [ ] Add Docker-based local ZTNet test harness + run CLI smoke test
