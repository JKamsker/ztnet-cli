# tRPC Commands — Task Checklist

## Phase 1: Session authentication infrastructure

- [x] Add `session_cookie` and `device_cookie` to `ProfileConfig` (`src/config.rs`)
- [x] Add `session_cookie` and `device_cookie` to `EffectiveConfig` (`src/context.rs`)
- [x] Add `SessionRequired` error variant (`src/error.rs`)
- [x] Add `auth login` CLI definition (`src/cli/auth.rs`)
- [x] Add `auth logout` CLI definition (`src/cli/auth.rs`)
- [x] Implement login flow: POST credentials, capture cookies (`src/app/auth.rs`)
- [x] Implement TOTP retry on `SecondFactorRequired` (`src/app/auth.rs`)
- [x] Implement logout: clear cookies from config (`src/app/auth.rs`)
- [x] Extend `auth show` to display session status (`src/app/auth.rs`)
- [x] Add session-aware tRPC helper (`src/app/trpc.rs`)
- [x] Refactor existing `trpc call` to use session from config when available

## Phase 2: Help UX

- [x] Add `[session auth]` suffix to `about` for all tRPC-only commands
- [x] Add `long_about` with session auth instructions

## Phase 3: Network commands (tRPC)

- [x] `network delete` CLI + implementation
- [x] `network routes list` CLI + implementation
- [x] `network routes add` CLI + implementation
- [x] `network routes remove` CLI + implementation
- [x] `network ip-pool list` CLI + implementation
- [x] `network ip-pool add` CLI + implementation
- [x] `network ip-pool remove` CLI + implementation
- [x] `network dns` CLI + implementation
- [x] `network ipv6` CLI + implementation
- [x] `network multicast` CLI + implementation
- [x] `network flow-rules get` CLI + implementation

## Phase 4: Member commands (tRPC)

- [x] `member add` CLI + implementation
- [x] `member tags list` CLI + implementation
- [x] `member tags set` CLI + implementation

## Phase 5: Admin commands (tRPC)

- [x] Add `Admin` top-level command (`src/cli.rs`, `src/app.rs`)
- [x] Create `src/cli/admin.rs` and `src/app/admin.rs`
- [x] `admin users list`
- [x] `admin users get`
- [x] `admin users delete`
- [x] `admin users update`
- [x] `admin backup list`
- [x] `admin backup create`
- [x] `admin backup download`
- [x] `admin backup restore`
- [x] `admin backup delete`
- [x] `admin mail setup`
- [x] `admin mail test`
- [x] `admin mail templates list/get/set`
- [x] `admin settings get`
- [x] `admin settings update`
- [x] `admin invites list`
- [x] `admin invites create`
- [x] `admin invites delete`

## Phase 6: Organization commands (tRPC)

- [x] `org users add`
- [x] `org users role`
- [x] `org invite create`
- [x] `org invite list`
- [x] `org invite delete`
- [x] `org invite send`
- [x] `org settings get`
- [x] `org settings update`
- [x] `org webhooks list`
- [x] `org webhooks add`
- [x] `org webhooks delete`
- [x] `org logs`

## Verification

- [x] `cargo build` succeeds
- [ ] `auth login` captures session cookie
- [ ] `auth show` displays session status
- [ ] `auth logout` clears session
- [ ] tRPC commands work with session auth
- [x] Missing session shows clear error message
- [x] `-h` shows `[session auth]` tags
- [ ] Smoke tests pass
