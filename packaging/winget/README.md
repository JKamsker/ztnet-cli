# WinGet notes

WinGet manifests are submitted to `microsoft/winget-pkgs`.

Suggested approach:

1. Ensure the GitHub Release includes the Windows asset:
   - `ztnet-<VERSION>-x86_64-pc-windows-msvc.zip`
   - and its `*.sha256`
2. Use `wingetcreate` to generate a portable manifest set using:
   - Installer URL (GitHub Release asset URL)
   - SHA256
3. Open a PR against `microsoft/winget-pkgs` with the generated manifests.

