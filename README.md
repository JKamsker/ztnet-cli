# ztnet-cli

Rust CLI for ZTNet.

## Dev quickstart (Windows)

- Start ZTNet locally: `powershell -File scripts\\ztnet-local.ps1 up`
- Build CLI: `cargo build`
- Bootstrap first user (fresh DB only):
  - `target\\debug\\ztnet.exe user create --email you@example.com --password Password123 --name "You" --generate-api-token --store-token --no-auth`
- Smoke test: set `ZTNET_SMOKE_EMAIL` / `ZTNET_SMOKE_PASSWORD`, then `powershell -File scripts\\smoke-test.ps1`
