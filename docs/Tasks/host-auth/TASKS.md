# Host-bound auth — Checklist

Source spec: `docs/Tasks/host-auth/plan.md`

- [x] Add plan + checklist
- [x] Add `host_defaults` to config schema and persistence
- [x] Add `auth hosts` CLI definitions (Clap)
- [x] Implement canonical host key + host-bound effective config resolution
- [x] Implement `auth hosts` command behaviors (list / set-default / unset-default)
- [x] Enforce explicit host for `auth set-token` + `auth login` and bind creds to host (+ auto host default if missing)
- [ ] Add unit tests for host key + profile selection + inference
- [ ] Update docs (README + docs/configuration.md + docs/commands.md + docs/api-reference.md)
