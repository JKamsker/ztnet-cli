# Plan: Add tRPC-based commands with user/password authentication

## Context

The ZTNet REST API (`/api/v1/`) covers basic CRUD for networks, members, and orgs. But many operations — network deletion, IPv6 config, managed routes, admin backup, org invites, etc. — are only available through tRPC endpoints. tRPC endpoints require a NextAuth session cookie (email/password login), not an API token. This plan adds:

1. A session-based auth flow (`auth login` / `auth logout`) that stores a NextAuth session cookie
2. Dedicated CLI commands for the most impactful tRPC-only operations
3. Clear UX: commands that need session auth say so in `-h` and in error messages

---

## Phase 1: Session authentication infrastructure

### 1.1 Add `auth login` command

**Flow:**
```
POST /api/auth/callback/credentials
Content-Type: application/x-www-form-urlencoded
Body: email=...&password=...&userAgent=ztnet-cli/0.1.0&totpCode=...
```

NextAuth responds with `302` + `Set-Cookie: next-auth.session-token=<JWT>; ...`.
The CLI must **not** follow the redirect and instead capture the `Set-Cookie` header.

- Reqwest with `redirect(Policy::none())` to capture the cookie from 302
- Parse `Set-Cookie` headers for `next-auth.session-token` and `next-auth.did-token`
- On `302` to `?error=SecondFactorRequired` → prompt for TOTP code interactively (or accept `--totp` flag)
- On `302` to `?error=IncorrectUsernamePassword` → clear error message

**CLI:**
```
ztnet auth login --email <EMAIL> --password <PASSWORD> [--totp <CODE>]
ztnet auth login --email <EMAIL> --password-stdin [--totp <CODE>]
```

**Storage:** Store the session cookie in the profile config alongside the API token:

```toml
[profiles.default]
host = "http://localhost:3000"
token = "sk_..."           # existing API token
session_cookie = "eyJ..."  # NEW: NextAuth JWT session cookie
device_cookie = "a1b2..."  # NEW: NextAuth device cookie
```

**Files to modify:**
- `src/cli/auth.rs` — add `Login` and `Logout` variants to `AuthCommand`
- `src/app/auth.rs` — implement login HTTP flow (separate reqwest client, no redirect follow)
- `src/config.rs` — add `session_cookie` and `device_cookie` fields to `ProfileConfig`
- `src/context.rs` — add `session_cookie` and `device_cookie` to `EffectiveConfig`

### 1.2 Add `auth logout` command

Clear stored session/device cookies from the profile config.

### 1.3 Extend `auth show`

Show whether a session cookie is present (e.g., `session: active` / `session: none`).

### 1.4 Add session-aware HTTP helper

Add a method to `HttpClient` (or a new helper in `src/app/common.rs`) that makes tRPC calls with session cookies automatically injected from the effective config. This replaces the current manual `--cookie` flow in `trpc call`.

Pattern:
```rust
fn trpc_request(&self, procedure: &str, input: Value, session: &str, device: &str) -> Result<Value>
```

### 1.5 Error handling for missing session

Add a new error variant:

```rust
#[error("this command requires session authentication\n\n  Run: ztnet auth login --email <EMAIL> --password <PASSWORD>\n\n  This command uses a tRPC endpoint that requires user credentials,\n  not an API token. See: ztnet auth login --help")]
SessionRequired,
```

Exit code: `3` (same bucket as auth errors).

Every tRPC-based command checks for `effective.session_cookie` and returns this error if missing. If a tRPC call returns `UNAUTHORIZED`, catch it and show the same message.

---

## Phase 2: CLI help UX — marking session-auth commands

### 2.1 Convention for help text

Commands requiring session auth get a suffix in their `about` text:

```
Commands:
  list        List all your networks
  create      Create a new network
  delete      Delete a network [session auth]
  routes      Manage network routes [session auth]
```

The `[session auth]` tag makes it instantly visible in `-h` output.

### 2.2 Long help includes instructions

Each session-auth command's `long_about` includes:

```
This command requires session authentication (email/password).
Run `ztnet auth login` first. API tokens are not supported for this operation.
```

---

## Phase 3: Network commands (tRPC)

### 3.1 `network delete <NETWORK> [--org <ORG>]`

- tRPC: `network.deleteNetwork`
- Requires confirmation (unless `-y`)
- Help: `[session auth]`

### 3.2 `network routes <NETWORK> <SUBCOMMAND> [--org <ORG>]`

Subcommands:
- `list` — show current managed routes
- `add --destination <CIDR> [--via <gateway>]`
- `remove --destination <CIDR>`

tRPC: `network.managedRoutes`

### 3.3 `network ip-pool <NETWORK> <SUBCOMMAND>`

Subcommands:
- `list` — show current IP assignments
- `add --start <IP> --end <IP>` or `--cidr <CIDR>`
- `remove --start <IP> --end <IP>` or `--cidr <CIDR>`

tRPC: `network.easyIpAssignment` / `network.advancedIpAssignment`

### 3.4 `network dns <NETWORK> [--org <ORG>]`

Options: `--domain <DOMAIN>`, `--servers <IP,...>`

tRPC: `network.dns`

### 3.5 `network ipv6 <NETWORK>`

Options: toggles for 6plane, rfc4193, auto-assign modes

tRPC: `network.ipv6`

### 3.6 `network multicast <NETWORK>`

Options: `--limit <N>`, `--enable/--disable`

tRPC: `network.multiCast`

### 3.7 `network flow-rules get <NETWORK>`

tRPC: `network.getFlowRule` (currently `network update --flow-rule` exists for setting via REST on org scope, but no GET for personal scope)

---

## Phase 4: Member commands (tRPC)

### 4.1 `member add <NETWORK> <NODE_ID>`

tRPC: `networkMember.create` — pre-provision a member by node ID

### 4.2 `member tags <NETWORK> <MEMBER>`

Subcommands: `list`, `set --tags <JSON>`

tRPC: `networkMember.Tags`

---

## Phase 5: Admin commands (tRPC)

### 5.1 `admin users`

Subcommands:
- `list` — list all platform users
- `get <USER>` — get user details
- `delete <USER>` — delete user (with confirmation)
- `update <USER> [--role <ROLE>] [--active/--inactive]`

tRPC: `admin.getUsers`, `admin.getUser`, `admin.deleteUser`, `admin.updateUser`, `admin.changeRole`

### 5.2 `admin backup`

Subcommands:
- `list` — list backups
- `create` — create backup
- `download <BACKUP> --out <PATH>` — download backup
- `restore <BACKUP>` — restore from backup (with confirmation)
- `delete <BACKUP>` — delete backup

tRPC: `admin.listBackups`, `admin.createBackup`, `admin.downloadBackup`, `admin.restoreBackup`, `admin.deleteBackup`

### 5.3 `admin mail`

Subcommands:
- `setup --host <SMTP_HOST> --port <PORT> --user <USER> --pass <PASS>` — configure SMTP
- `test --to <EMAIL>` — send test email
- `templates list` / `templates get <NAME>` / `templates set <NAME> --file <PATH>`

tRPC: `admin.setMail`, `admin.sendTestMail`, `admin.getMailTemplates`, `admin.setMailTemplates`

### 5.4 `admin settings`

Subcommands:
- `get` — show global settings
- `update [--registration-enabled] [--...]` — update global options

tRPC: `settings.getAllOptions`, `admin.updateGlobalOptions`

### 5.5 `admin invites`

Subcommands:
- `list` — list invitation links
- `create` — generate invite link
- `delete <INVITE>` — delete invite link

tRPC: `admin.generateInviteLink`, `admin.getInvitationLink`, `admin.deleteInvitationLink`

---

## Phase 6: Organization commands (tRPC)

### 6.1 `org users add <ORG> --email <EMAIL>`

tRPC: `org.addUser`

### 6.2 `org users role <ORG> <USER> --role <ROLE>`

tRPC: `org.changeUserRole`

### 6.3 `org invite`

Subcommands:
- `create <ORG>` — generate invite link
- `list <ORG>` — list pending invites
- `delete <ORG> <INVITE>` — cancel invite
- `send <ORG> --email <EMAIL>` — invite by email

tRPC: `org.generateInviteLink`, `org.getInvites`, `org.deleteInvite`, `org.inviteUserByMail`

### 6.4 `org settings <ORG>`

Subcommands: `get`, `update [--name ...] [--description ...]`

tRPC: `org.getOrganizationSettings`, `org.updateOrganizationSettings`

### 6.5 `org webhooks <ORG>`

Subcommands: `list`, `add --url <URL>`, `delete <WEBHOOK>`

tRPC: `org.addOrgWebhooks`, `org.getOrgWebhooks`, `org.deleteOrgWebhooks`

### 6.6 `org logs <ORG>`

List activity/audit logs.

tRPC: `org.getLogs`

---

## Implementation order

| Step | What | Priority |
|------|------|----------|
| 1 | `auth login` / `auth logout` + session storage + error handling | **Must have** |
| 2 | tRPC helper (auto-inject session cookie) | **Must have** |
| 3 | `network delete` | **High** (most requested missing command) |
| 4 | `network routes`, `network ip-pool`, `network dns` | **High** |
| 5 | `member add`, `member tags` | **Medium** |
| 6 | `admin users`, `admin backup` | **Medium** |
| 7 | `network ipv6`, `network multicast`, `network flow-rules get` | **Medium** |
| 8 | `admin mail`, `admin settings`, `admin invites` | **Lower** |
| 9 | `org invite`, `org settings`, `org webhooks`, `org logs`, `org users add/role` | **Lower** |

---

## Key files to create/modify

| File | Change |
|------|--------|
| `src/config.rs` | Add `session_cookie`, `device_cookie` to `ProfileConfig` |
| `src/context.rs` | Add `session_cookie`, `device_cookie` to `EffectiveConfig` |
| `src/error.rs` | Add `SessionRequired` variant |
| `src/cli/auth.rs` | Add `Login`, `Logout` subcommands |
| `src/app/auth.rs` | Implement login flow (credentials POST, cookie capture, TOTP retry) |
| `src/app/trpc.rs` | Refactor into reusable `trpc_call()` that auto-injects session from config |
| `src/cli.rs` | Add `Admin` top-level command |
| `src/cli/admin.rs` | **New** — admin subcommand definitions |
| `src/app/admin.rs` | **New** — admin business logic |
| `src/cli/network.rs` | Add `Delete`, `Routes`, `IpPool`, `Dns`, `Ipv6`, `Multicast` subcommands |
| `src/app/network.rs` | Implement tRPC-based network operations |
| `src/cli/org.rs` | Add `Invite`, `Settings`, `Webhooks`, `Logs` subcommands; extend `Users` |
| `src/app/org.rs` | Implement tRPC-based org operations |
| `src/cli/network.rs` | Add `Add`, `Tags` to member subcommands |
| `src/app/member.rs` | Implement tRPC-based member operations |

---

## Deliverables

- Implementation across all files listed above
- `docs/Tasks/tRPC/plan.md` — this plan, saved in the repo
- `docs/Tasks/tRPC/TASKS.md` — checklist-style tracking document

---

## Verification

1. Start local ZTNet: `powershell -File scripts/ztnet-local.ps1 up`
2. Bootstrap user: `ztnet user create --email test@test.com --password Test1234 --name Test --generate-api-token --store-token --no-auth`
3. Test session login: `ztnet auth login --email test@test.com --password Test1234`
4. Verify `auth show` shows both token and session
5. Test tRPC command: `ztnet network delete <id> -y`
6. Test error UX: clear session with `auth logout`, retry `network delete` → expect clear "run auth login" message
7. Verify `-h` shows `[session auth]` tags on tRPC commands
8. Run smoke tests: `powershell -File scripts/smoke-test.ps1`
