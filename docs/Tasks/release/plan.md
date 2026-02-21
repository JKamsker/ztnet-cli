# GitHub CI + Releases + crates.io + Windows package support

## Summary
Add a GitHub Actions CI pipeline that runs Rust tests on PRs and pushes, and a release pipeline that publishes a GitHub Release from `master` when the crate version changes (new tag). After CI is green, publish to crates.io (automated via Actions with `CARGO_REGISTRY_TOKEN`). Prepare Scoop and WinGet support by providing release asset naming + manifest templates.

## Goals
- CI on GitHub: `cargo test --locked` on Linux/Windows/macOS for pushes + PRs.
- Release from `master`: build release binaries, publish a GitHub Release, and attach checksums.
- crates.io publish: publish on release (requires `CARGO_REGISTRY_TOKEN` secret).
- Scoop + WinGet: stable Windows release assets and template manifests to submit to a Scoop bucket and `winget-pkgs`.

## Non-goals
- Automatically submitting PRs to external Scoop buckets or `microsoft/winget-pkgs`.
- Complex installer generation (MSI) unless needed later.

## Versioning & release trigger
- Release workflow runs on pushes to `master`.
- It determines the current `Cargo.toml` package version and uses tag `v<version>`.
- If tag `v<version>` already exists, it skips release/publish steps.
- If tag doesn’t exist, it creates a GitHub Release with that tag and assets.

## Required secrets
- `CARGO_REGISTRY_TOKEN` (crates.io API token) for `cargo publish`.

