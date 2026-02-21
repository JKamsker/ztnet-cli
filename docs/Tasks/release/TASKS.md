# Release pipeline — Checklist

Source spec: `docs/Tasks/release/plan.md`

- [x] Add plan + checklist
- [x] Add crates.io-ready package metadata (Cargo.toml + LICENSE)
- [x] Fix README license section (AGPL)
- [x] Add GitHub Actions CI (`cargo test`) for PRs and pushes
- [x] Add GitHub Actions release workflow (build + GitHub Release on `master`)
- [x] Add crates.io publish step to release workflow (token-based)
- [x] Prepare Scoop + WinGet manifests/templates and packaging docs
- [ ] Configure git remote and push to GitHub
