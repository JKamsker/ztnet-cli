# Development Guide

How to build, test, and contribute to ztnet-cli.

## Prerequisites

- **Rust** 1.81+ (edition 2024)
- **Docker** and **Docker Compose** (for local ZTNet instance)
- **PowerShell** (for Windows scripts) or adapt the commands for your shell

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run directly
cargo run -- network list
```

The binary is produced at:
- Debug: `target/debug/ztnet` (or `ztnet.exe` on Windows)
- Release: `target/release/ztnet` (or `ztnet.exe` on Windows)

## Local ZTNet with Docker

The project includes a Docker Compose setup for running a full ZTNet stack locally via the `external/ztnet` git submodule.

### Start the stack

```powershell
# Initialize the submodule (first time)
git submodule update --init

# Start ZTNet + PostgreSQL + ZeroTier controller
powershell -File scripts/ztnet-local.ps1 up
```

ZTNet will be available at `http://localhost:3000`.

### Manage the stack

```powershell
# View running containers
powershell -File scripts/ztnet-local.ps1 ps

# View logs
powershell -File scripts/ztnet-local.ps1 logs
powershell -File scripts/ztnet-local.ps1 logs -Follow

# Stop
powershell -File scripts/ztnet-local.ps1 down

# Stop and remove volumes (full reset)
powershell -File scripts/ztnet-local.ps1 down -Volumes
```

### Bootstrap the first user

On a fresh database, create the first user without authentication:

```bash
target/debug/ztnet user create \
  --email admin@example.com \
  --password Password123 \
  --name "Admin" \
  --generate-api-token \
  --store-token \
  --no-auth
```

This creates the user, generates an API token, and saves it to your config so subsequent commands are authenticated.

## Smoke tests

The integration smoke test verifies the CLI against a running ZTNet instance.

### Running

```powershell
# Set credentials (optional - defaults to test@ztnet.local / TestPassword123!)
$env:ZTNET_SMOKE_EMAIL = "admin@example.com"
$env:ZTNET_SMOKE_PASSWORD = "Password123"
$env:ZTNET_SMOKE_NAME = "Admin"

# Run the smoke test
powershell -File scripts/smoke-test.ps1
```

### What it does

1. Waits for ZTNet to be ready (up to 3 minutes)
2. Bootstraps a user (if email/password provided)
3. Validates authentication with `auth test`
4. Creates a timestamped test network
5. Fetches network details
6. Exports hosts in JSON format

### Full local test cycle

```powershell
# 1. Start ZTNet
powershell -File scripts/ztnet-local.ps1 up

# 2. Build
cargo build

# 3. Run smoke tests
powershell -File scripts/smoke-test.ps1

# 4. Tear down
powershell -File scripts/ztnet-local.ps1 down -Volumes
```

## Project architecture

```
src/
 ├── main.rs           Entry point (tokio async runtime)
 ├── cli.rs            Clap parser, global options, Command enum
 ├── cli/              CLI definitions (pure argument parsing, no logic)
 │   ├── auth.rs
 │   ├── config_cmd.rs
 │   ├── user.rs
 │   ├── org.rs
 │   ├── network.rs
 │   ├── stats.rs
 │   ├── planet.rs
 │   ├── export.rs
 │   ├── api.rs
 │   ├── trpc.rs
 │   └── completion.rs
 ├── app.rs            Main dispatcher (routes commands to handlers)
 ├── app/              Business logic (one file per command group)
 │   ├── auth.rs       Token and profile management
 │   ├── config_cmd.rs Config file operations
 │   ├── user.rs       User creation
 │   ├── org.rs        Organization operations
 │   ├── network.rs    Network CRUD
 │   ├── member.rs     Member CRUD
 │   ├── stats.rs      Statistics
 │   ├── planet.rs     Planet file download
 │   ├── export.rs     Hosts/CSV/JSON export
 │   ├── api.rs        Raw HTTP requests
 │   ├── trpc.rs       tRPC procedure calls
 │   ├── common.rs     Shared I/O and formatting utilities
 │   └── resolve.rs    Name-to-ID resolution
 ├── config.rs         TOML config file loading/saving
 ├── context.rs        Config precedence resolution
 ├── http.rs           HTTP client (auth, retries, dry-run)
 ├── output.rs         Output formatting (table, JSON, YAML, raw)
 └── error.rs          Error types and exit codes
```

### Key design decisions

**Separation of CLI and logic.** The `src/cli/` directory contains only Clap derive structs for argument parsing. The `src/app/` directory contains the actual business logic. This keeps the two concerns decoupled and easy to test independently.

**Config precedence.** Configuration is resolved through a clear chain: CLI flags override environment variables, which override the config file, which provides defaults. The `context.rs` module handles this merging.

**Scoping model.** The same commands work in both personal and organization scope. When `--org` is provided (via flag, env, or context default), API calls are routed to `/api/v1/org/{orgId}/...` instead of `/api/v1/...`.

**Name resolution.** Networks and organizations can be referenced by name. The `resolve.rs` module fetches the list, matches by name, and returns the ID. Ambiguous matches (multiple results) produce a clear error.

**HTTP resilience.** The HTTP client in `http.rs` handles retries with exponential backoff, rate limit detection via `Retry-After` headers, and dry-run mode. All API calls go through this single client.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` + `clap_complete` | CLI argument parsing and shell completions |
| `tokio` | Async runtime |
| `reqwest` | HTTP client (with rustls-tls) |
| `serde` + `serde_json` + `serde_yaml` | Serialization |
| `toml` | Config file format |
| `comfy-table` | ASCII table rendering |
| `thiserror` | Error type derivation |
| `humantime` | Duration parsing (e.g., `30s`) |
| `url` | URL parsing and joining |

## Release automation

Releases are automated on GitHub:

- Every push to `master` runs `.github/workflows/release.yml`, which:
  - skips automatically if there are no changes under `src/` since the latest `v*` tag (unless triggered via `workflow_dispatch`)
  - bumps the patch version in `Cargo.toml` (and the root package version in `Cargo.lock`)
  - updates the Scoop manifest in-repo (`bucket/ztnet.json`)
  - commits all of the above in a single `chore(release): v<version>` commit and tags it as `v<version>`
  - runs `cargo test --locked`, builds release binaries, publishes a GitHub Release, and publishes to crates.io when `CARGO_REGISTRY_TOKEN` is set
- WinGet PR automation (optional) is applied by the `winget` job in `.github/workflows/release.yml` (requires `WINGET_TOKEN` (classic PAT) and an existing base manifest in `microsoft/winget-pkgs`; the job skips until the initial submission is merged).
- Chocolatey publishing (optional) is applied by the `chocolatey` job in `.github/workflows/release.yml` (requires `CHOCO_API_KEY`).
