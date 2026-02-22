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

This repo includes a GitHub Actions workflow that can open WinGet update PRs automatically on every GitHub Release:

- Workflow: `.github/workflows/winget.yml`
- Action: `vedantmgoyal9/winget-releaser`
- Identifier: `JKamsker.ZTNetCLI`

For automated releases created by `.github/workflows/release.yml`, the WinGet update is run from the `winget` job inside that same workflow (because GitHub does not trigger `on: release` workflows for releases created with `GITHUB_TOKEN`).

Prerequisites (per action docs):
- At least one version of the package must already exist in `microsoft/winget-pkgs` (the action uses it as a base).
- A fork of `microsoft/winget-pkgs` must exist under the same account/org as this repo (or configure the action accordingly).
- A classic PAT secret `WINGET_TOKEN` (scope: `public_repo`) must be configured in this repo.
