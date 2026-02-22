# Chocolatey

This repo can publish `ztnet` to Chocolatey (community feed).

## Package sources

Package sources live in `packaging/chocolatey/ztnet/`.

The installer downloads the Windows ZIP asset from the GitHub Release for the package version and shims `ztnet.exe` as `ztnet`.

## Automation

`.github/workflows/release.yml` includes an optional Chocolatey publish job:

- Secret: `CHOCO_API_KEY`
- Source: `https://push.chocolatey.org/`
- Package id: `ztnet`

If `CHOCO_API_KEY` is missing/empty, the job skips.

## Manual publish

1. Get your API key from `https://push.chocolatey.org/account`.
2. Pack:
   - `choco pack packaging/chocolatey/ztnet/ztnet.nuspec --version <VERSION>`
3. Push:
   - `choco push ztnet.<VERSION>.nupkg --source https://push.chocolatey.org/ --api-key <KEY>`

