# Packaging (Scoop + WinGet)

This repo publishes GitHub Release assets that can be used by Windows package managers.

It also ships an in-repo Scoop bucket manifest at `bucket/ztnet.json` that is automatically updated on each GitHub Release.

## Release assets

The GitHub Release workflow uploads these assets (names include the Rust host target triple):

- Windows: `ztnet-<VERSION>-x86_64-pc-windows-msvc.zip`
- Linux: `ztnet-<VERSION>-x86_64-unknown-linux-gnu.tar.gz`
- macOS: `ztnet-<VERSION>-*-apple-darwin.tar.gz`
- Checksums: each asset also has a sibling `*.sha256` file

## Scoop

### Install from this repo (bucket)

This repository can be used directly as a Scoop bucket:

```powershell
scoop bucket add ztnet-cli https://github.com/JKamsker/ztnet-cli
scoop install ztnet-cli/ztnet
```

The manifest is `bucket/ztnet.json` and is kept up-to-date by `.github/workflows/scoop.yml` when a GitHub Release is published.

### Submit to a separate bucket

If you prefer to publish to a dedicated Scoop bucket repository, use the template in `packaging/scoop/ztnet.json.template` and fill:

- `version`
- `url` (points to the GitHub Release asset)
- `hash` (from the `*.sha256` file, or computed locally)

Example URL format:

```
https://github.com/JKamsker/ztnet-cli/releases/download/v<VERSION>/ztnet-<VERSION>-x86_64-pc-windows-msvc.zip
```

## WinGet

WinGet manifests are submitted to `microsoft/winget-pkgs` (not this repo).

Recommended workflow:

1. Create a GitHub Release with the Windows zip asset.
2. Use `wingetcreate` (official helper) to generate manifests from the installer URL + SHA256.
3. Submit a PR to `microsoft/winget-pkgs`.

Notes:
- A `.zip` asset is typically packaged as a “portable” app in WinGet. `wingetcreate` guides the exact schema fields required.
- This repo also contains `.github/workflows/winget.yml`, which can open WinGet update PRs automatically on each GitHub Release. See `packaging/winget/README.md`.
