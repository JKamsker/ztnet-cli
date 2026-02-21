# Auto release on every `master` push — Checklist

Source spec: `docs/Tasks/auto-release-every-push/plan.md`

- [x] Add plan + checklist
- [x] Add `version-bump` workflow (patch bump + tag) on `master` push
- [ ] Update release workflow to trigger on tags, run tests, and publish on every new tag
- [ ] Add Scoop bucket manifest in-repo + automate updates on release
- [ ] Add WinGet workflow to open PRs on every GitHub Release (token-gated)
- [ ] Update docs (release process + Scoop/WinGet usage)
