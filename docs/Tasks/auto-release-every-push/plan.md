# Auto release on every `master` push (Cargo + GitHub Release + Scoop + WinGet)

## Summary
Automate a full release on every push to `master`:

1. Bump `Cargo.toml` patch version (and `Cargo.lock` root package version)
2. Commit the bump back to `master`
3. Create a `v<version>` git tag
4. Build + test + publish:
   - Create a GitHub Release with cross-platform binaries + sha256 files
   - Publish the crate to crates.io
   - Update Scoop manifest (in-repo bucket)
   - Open a WinGet submission PR (requires PAT + existing package in winget-pkgs)

## Design
- **Version bump** happens in a dedicated workflow triggered by pushes to `master`.
  - It skips runs authored by `github-actions[bot]` (avoids infinite loops).
  - It bumps **patch** on every non-bot push.
  - It commits `Cargo.toml` + `Cargo.lock` changes and tags the bump commit as `v<new_version>`.
- **Release** happens via `workflow_dispatch` triggered by the version bump workflow (and also supports manual tag pushes `v*`) and:
  - Runs `cargo test --locked`
  - Builds and packages release binaries for Windows/Linux/macOS
  - Creates a GitHub Release for the tag (or no-ops if it already exists)
  - Publishes to crates.io if `CARGO_REGISTRY_TOKEN` is present
- **Scoop** is maintained as an in-repo bucket at `bucket/ztnet.json`, updated on releases.
- **WinGet** is best-effort automation:
  - Uses `vedantmgoyal9/winget-releaser`
  - Requires a classic PAT secret (e.g. `WINGET_TOKEN`) and an existing base submission in `microsoft/winget-pkgs`

## Required secrets
- `CARGO_REGISTRY_TOKEN` (crates.io API token)
- `WINGET_TOKEN` (classic PAT with `public_repo` scope) — optional, for WinGet PR automation
