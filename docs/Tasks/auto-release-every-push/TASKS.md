# Auto release on every `master` push — Checklist

Source spec: `docs/Tasks/auto-release-every-push/plan.md`

- [x] Add plan + checklist
- [x] Add `version-bump` workflow (patch bump + tag) on `master` push
- [x] Fix `version-bump` workflow heredoc indentation
- [x] Fix `version-bump` workflow newline writing (repair `Cargo.toml`/`Cargo.lock`)
- [x] Trigger release workflow from `version-bump` (workflow_dispatch)
- [x] Update release workflow to trigger on tags, run tests, and publish on every new tag
- [x] Add Scoop bucket manifest in-repo + automate updates on release
- [x] Add WinGet workflow to open PRs on every GitHub Release (token-gated)
- [x] Update docs (release process + Scoop/WinGet usage)
