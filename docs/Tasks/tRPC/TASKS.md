# tRPC Commands — Task Checklist

## Phase 1: Session authentication infrastructure

- [ ] Add `session_cookie` and `device_cookie` to `ProfileConfig` (`src/config.rs`)
- [ ] Add `session_cookie` and `device_cookie` to `EffectiveConfig` (`src/context.rs`)
- [ ] Add `SessionRequired` error variant (`src/error.rs`)
- [ ] Add `auth login` CLI definition (`src/cli/auth.rs`)
- [ ] Add `auth logout` CLI definition (`src/cli/auth.rs`)
- [ ] Implement login flow: POST credentials, capture cookies (`src/app/auth.rs`)
- [ ] Implement TOTP retry on `SecondFactorRequired` (`src/app/auth.rs`)
- [ ] Implement logout: clear cookies from config (`src/app/auth.rs`)
- [ ] Extend `auth show` to display session status (`src/app/auth.rs`)
- [ ] Add session-aware tRPC helper (`src/app/trpc.rs`)
- [ ] Refactor existing `trpc call` to use session from config when available

## Phase 2: Help UX

- [ ] Add `[session auth]` suffix to `about` for all tRPC-only commands
- [ ] Add `long_about` with session auth instructions

## Phase 3: Network commands (tRPC)

- [ ] `network delete` CLI + implementation
- [ ] `network routes list` CLI + implementation
- [ ] `network routes add` CLI + implementation
- [ ] `network routes remove` CLI + implementation
- [ ] `network ip-pool list` CLI + implementation
- [ ] `network ip-pool add` CLI + implementation
- [ ] `network ip-pool remove` CLI + implementation
- [ ] `network dns` CLI + implementation
- [ ] `network ipv6` CLI + implementation
- [ ] `network multicast` CLI + implementation
- [ ] `network flow-rules get` CLI + implementation

## Phase 4: Member commands (tRPC)

- [ ] `member add` CLI + implementation
- [ ] `member tags list` CLI + implementation
- [ ] `member tags set` CLI + implementation

## Phase 5: Admin commands (tRPC)

- [ ] Add `Admin` top-level command (`src/cli.rs`, `src/app.rs`)
- [ ] Create `src/cli/admin.rs` and `src/app/admin.rs`
- [ ] `admin users list`
- [ ] `admin users get`
- [ ] `admin users delete`
- [ ] `admin users update`
- [ ] `admin backup list`
- [ ] `admin backup create`
- [ ] `admin backup download`
- [ ] `admin backup restore`
- [ ] `admin backup delete`
- [ ] `admin mail setup`
- [ ] `admin mail test`
- [ ] `admin mail templates list/get/set`
- [ ] `admin settings get`
- [ ] `admin settings update`
- [ ] `admin invites list`
- [ ] `admin invites create`
- [ ] `admin invites delete`

## Phase 6: Organization commands (tRPC)

- [ ] `org users add`
- [ ] `org users role`
- [ ] `org invite create`
- [ ] `org invite list`
- [ ] `org invite delete`
- [ ] `org invite send`
- [ ] `org settings get`
- [ ] `org settings update`
- [ ] `org webhooks list`
- [ ] `org webhooks add`
- [ ] `org webhooks delete`
- [ ] `org logs`

## Verification

- [ ] `cargo build` succeeds
- [ ] `auth login` captures session cookie
- [ ] `auth show` displays session status
- [ ] `auth logout` clears session
- [ ] tRPC commands work with session auth
- [ ] Missing session shows clear error message
- [ ] `-h` shows `[session auth]` tags
- [ ] Smoke tests pass
