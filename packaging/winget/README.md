# WinGet notes

WinGet manifests are submitted to `microsoft/winget-pkgs`.

Suggested approach:

1. Ensure the GitHub Release includes the Windows asset:
   - `ztnet-<VERSION>-x86_64-pc-windows-msvc.zip`
   - and its `*.sha256`
2. Create the initial submission in `microsoft/winget-pkgs` using `wingetcreate` to generate a portable manifest set from:
   - Installer URL (GitHub Release asset URL)
   - SHA256
3. Open a PR against `microsoft/winget-pkgs` with the generated manifests.

## Automation (recommended after initial submission)

This repo can open WinGet update PRs automatically on every automated release:

- Workflow: `.github/workflows/release.yml` (`winget` job)
- Action: `vedantmgoyal9/winget-releaser`
- Identifier: `JKamsker.ZTNetCLI`

Note: `.github/workflows/winget.yml` exists but is not reliable for automated releases created with `GITHUB_TOKEN` (GitHub does not trigger `on: release` workflows for those releases).

Prerequisites (per action docs):
- At least one version of the package must already exist in `microsoft/winget-pkgs` (the action uses it as a base).
- A fork of `microsoft/winget-pkgs` must exist under the same account/org as this repo (or configure the action accordingly).
- A classic PAT secret `WINGET_TOKEN` (scope: `public_repo`) must be configured in this repo.

Implementation detail:
- The `winget` job in `.github/workflows/release.yml` also checks if the package exists in `microsoft/winget-pkgs` yet and skips until the initial submission PR is merged (to keep releases green).
- Because the workflow runs on `push` to `master`, the action must be given the actual release tag (for example `v0.1.4`) via `release-tag` instead of defaulting to `master`.
- The `installers-regex` is a regex; use `\.zip$` (dot escaped) to match zip assets.

### Token creation

GitHub classic PATs cannot be created via `gh` CLI. Create one in the GitHub UI, then store it as `WINGET_TOKEN`.
