# Auto release on every `master` push (Cargo + GitHub Release + Scoop + WinGet)

## Summary
Automate a full release on every push to `master`:

1. Determine the next patch version
2. Build + test + package release assets
3. Update manifests in one commit:
   - `Cargo.toml` (version)
   - `Cargo.lock` (root package version)
   - `bucket/ztnet.json` (Scoop manifest)
4. Create a `v<version>` git tag and publish:
   - Create a GitHub Release with cross-platform binaries + sha256 files
   - Publish the crate to crates.io
   - Open a WinGet submission PR (requires PAT + existing package in winget-pkgs)

## Design
- **Release** happens in a single workflow (`.github/workflows/release.yml`) triggered by pushes to `master` (and also supports manual tag pushes `v*`) and:
  - Skips runs authored by `github-actions[bot]` (avoids infinite loops).
  - Bumps **patch** on every non-bot push to `master`.
  - Runs `cargo test --locked`
  - Builds and packages release binaries for Windows/Linux/macOS
  - Creates a GitHub Release for the tag (or no-ops if it already exists)
  - Publishes to crates.io if `CARGO_REGISTRY_TOKEN` is present
- **Scoop** is maintained as an in-repo bucket at `bucket/ztnet.json`, updated as part of the same release bump commit.
- **WinGet** is best-effort automation:
  - Uses `vedantmgoyal9/winget-releaser`
  - Requires a classic PAT secret (e.g. `WINGET_TOKEN`) and an existing base submission in `microsoft/winget-pkgs`

## Required secrets
- `CARGO_REGISTRY_TOKEN` (crates.io API token)
- `WINGET_TOKEN` (classic PAT with `public_repo` scope) — optional, for WinGet PR automation
