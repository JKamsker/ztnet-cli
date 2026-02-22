# Chocolatey publishing

Goal: publish `ztnet` to Chocolatey (community feed) and optionally automate publishing on each GitHub release.

Notes:
- Chocolatey publish requires an API key from the Chocolatey account page.
- This repo already produces a Windows portable ZIP asset on GitHub Releases:
  `ztnet-<VERSION>-x86_64-pc-windows-msvc.zip` + `*.sha256`.
- The Chocolatey package will install by downloading that ZIP and shimming `ztnet.exe` as `ztnet`.
- Automation should be opt-in (only runs when `CHOCO_API_KEY` secret is present) to keep releases green by default.

