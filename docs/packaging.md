# Packaging (Scoop + WinGet)

This repo publishes GitHub Release assets that can be used by Windows package managers.

## Release assets

The GitHub Release workflow uploads these assets (names include the Rust host target triple):

- Windows: `ztnet-<VERSION>-x86_64-pc-windows-msvc.zip`
- Linux: `ztnet-<VERSION>-x86_64-unknown-linux-gnu.tar.gz`
- macOS: `ztnet-<VERSION>-*-apple-darwin.tar.gz`
- Checksums: each asset also has a sibling `*.sha256` file

## Scoop

Scoop manifests live in a Scoop bucket repo (not this repo). Use the template in `packaging/scoop/ztnet.json.template` and fill:

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

