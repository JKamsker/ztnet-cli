Below is a **full proposed CLI UX spec** for a `ztnet` CLI that covers **every endpoint you inventoried**  and matches the official ZTNet REST docs (base path `/api/v1`, auth header `x-ztnet-auth`, rate-limited). ([ZTNET][1])

I’m deliberately copying a few “proven” UX conventions:

* **Global `--json` and `--yes/-y`** (automation-friendly) like Railway CLI. ([Railway Docs][2])
* **Config file + env vars** (and flag overrides) like sentry-cli. ([Sentry Documentation][3])
* Compatibility with the env vars already used in ZTNet’s own `zt2hosts` script (`ZTNET_API_TOKEN`, `API_ADDRESS`). ([ZTNET][4])

---

## Command tree

This is what `ztnet --help` should look like (Clap-style):

```text
ztnet 0.1.0
ZTNet CLI — manage ZeroTier networks via ZTNet

USAGE:
  ztnet [GLOBAL OPTIONS] <COMMAND> [ARGS...]

GLOBAL OPTIONS:
  -H, --host <URL>                 ZTNet base URL (e.g. http://localhost:3000)
                                   Env: ZTNET_HOST, API_ADDRESS
  -t, --token <TOKEN>              API token (x-ztnet-auth)
                                   Env: ZTNET_API_TOKEN, ZTNET_TOKEN
      --profile <NAME>             Named profile from config (default: "default")
      --org <ORG>                  Organization scope (orgId or resolvable name)
      --network <NETWORK>          Default network (id or resolvable name)
      --json                       Output JSON (Railway-style)
  -o, --output <FORMAT>            table|json|yaml|raw (default: table)
      --no-color                   Disable ANSI colors
      --quiet                      Only print machine output (no spinners/prompts)
  -v, --verbose                    Verbose logs (repeatable: -vv, -vvv)
      --timeout <DURATION>         HTTP timeout (default: 30s)
      --retries <N>                Retry count for transient errors (default: 3)
      --dry-run                    Print the HTTP request and exit (no network calls)
  -y, --yes                        Skip confirmation prompts
  -h, --help                       Print help
  -V, --version                    Print version

COMMANDS:
  auth         Manage tokens, profiles, and connectivity checks
  config       View/edit config, defaults, and context
  user         Create platform users (bootstrap/admin)
  org          List/get orgs; list org users
  network      List/create/get/update networks (personal or org scope)
  member       Shortcut alias: network member ...
  stats        Admin stats (GET /api/v1/stats)
  planet       Download planet file (GET /api/planet)
  export       Generate derived artifacts (hosts file, etc.)
  api          Raw HTTP for any endpoint (escape hatch)
  trpc         Experimental: call UI tRPC procedures (requires NextAuth cookie)
  completion   Generate shell completions

Run 'ztnet <COMMAND> --help' for details.
```

Notes:

* `--json` exists as a dedicated global switch because it’s a well-known convention in other Rust CLIs (e.g., Railway). ([Railway Docs][2])
* `--yes/-y` is global so destructive operations (`member delete`) are scriptable. ([Railway Docs][2])

---

## Identity and scoping model

ZTNet has two “scopes” in the REST API inventory:

* **Personal scope** under `/api/v1/network/...` ([ZTNET][5])
* **Organization scope** under `/api/v1/org/:orgId/...` ([ZTNET][6])

UX rule:

* If `--org` is **not set**, network commands target **personal** endpoints.
* If `--org <orgId>` **is set**, the same commands target **org** endpoints.

This avoids having two parallel trees like `personal network ...` vs `org network ...`.

---

## Auth, config, and precedence

### Authentication header

All `/api/v1/...` REST calls send:

* Header: `x-ztnet-auth: <token>` ([ZTNET][1])
* Rate limit is documented as **50 requests/minute** for these REST APIs; CLI should retry/back off on `429`. ([ZTNET][1])

### Config sources and precedence

Modeled after sentry-cli’s config/env approach: ([Sentry Documentation][3])

1. CLI flags
2. Environment variables
3. Config file
4. Defaults

### Environment variables

Support both “official-doc-used” vars and “cli-native” vars:

* `ZTNET_API_TOKEN` (documented in ZT2Hosts) ([ZTNET][4])
* `API_ADDRESS` (documented in ZT2Hosts) ([ZTNET][4])
* `ZTNET_HOST` (more explicit alternative)
* `ZTNET_TOKEN` (more explicit alternative)
* `ZTNET_OUTPUT` (`table|json|yaml|raw`)
* `ZTNET_PROFILE`

### Config file layout

Default file path (cross-platform via XDG / platform dirs):

* Linux: `~/.config/ztnet/config.toml`
* macOS: `~/Library/Application Support/ztnet/config.toml`
* Windows: `%APPDATA%\ztnet\config.toml`

Example:

```toml
active_profile = "default"

[profiles.default]
host = "http://localhost:3000"
token = "REDACTED"
default_org = ""
default_network = ""
output = "table"
timeout = "30s"
retries = 3

[profiles.prod]
host = "https://ztnet.example.com"
token = "REDACTED"
default_org = "org_123"
output = "json"
```

---

## `auth` commands

### `ztnet auth --help`

```text
USAGE:
  ztnet auth <SUBCOMMAND>

SUBCOMMANDS:
  set-token        Save token to config (or print instructions)
  unset-token      Remove token from config
  show             Print current auth context (host/profile/org)
  test             Validate token by performing a harmless API call
  profiles         Manage named profiles
```

#### `auth set-token`

```text
USAGE:
  ztnet auth set-token [--profile <NAME>] [--stdin | <TOKEN>]

OPTIONS:
  --stdin          Read token from STDIN (avoids shell history)
```

Behavior:

* Writes token into config under that profile.

#### `auth test`

```text
USAGE:
  ztnet auth test [--org <ORG>]

BEHAVIOR:
  - Without --org: calls GET /api/v1/network
  - With --org:    calls GET /api/v1/org
```

Cites for the endpoints: personal networks list and org list. ([ZTNET][5])

---

## `config` commands

### `ztnet config --help`

```text
USAGE:
  ztnet config <SUBCOMMAND>

SUBCOMMANDS:
  path           Print config file path
  get            Get a config value (supports dotted keys)
  set            Set a config value
  unset          Remove a config value
  list           Print full effective config (merged + redacted)
  context        Manage default org/network context
```

### `ztnet config context --help`

```text
USAGE:
  ztnet config context <SUBCOMMAND>

SUBCOMMANDS:
  show           Show default org/network for current profile
  set            Set defaults (persist in config)
  clear          Clear defaults
```

`context set`:

```text
USAGE:
  ztnet config context set [--org <ORG>] [--network <NETWORK>]
```

---

## `user` commands (bootstrap/admin)

This maps to **POST `/api/v1/user`**.

Important UX detail from official docs:

* **If no users exist yet, no API key is required**, and **the first created user becomes admin**. Otherwise you need an API key in the request header. ([ZTNET][7])

### `ztnet user --help`

```text
USAGE:
  ztnet user <SUBCOMMAND>

SUBCOMMANDS:
  create         Create a new ZTNet user (bootstrap/admin-only after bootstrap)
```

### `ztnet user create`

```text
USAGE:
  ztnet user create --email <EMAIL> --password <PASSWORD> --name <NAME>
                    [--expires-at <ISO8601>] [--generate-api-token]
                    [--store-token] [--print-token]

OPTIONS:
  --expires-at <ISO8601>       Optional token expiry timestamp
  --generate-api-token         Ask server to include a token in the response
  --sto:contentReference[oaicite:17]{index=17}   If token returned, save it into config profile
  --print-token                If token returned, print to stdout
  --no-auth                    Force no x-ztnet-auth header (bootstrap attempt)
```

Request fields (from your inventory): email, password, name, expiresAt, generateApiToken.

Docs hint that enabling token generation returns a time-limited token (not always shown in the HTML view, but appears in indexed snippet). ([ZTNET][8])

---

## `org` commands

Endpoints:

* `GET /api/v1/org` list orgs ([ZTNET][6])
* `GET /api/v1/org/:orgId` get org info ([ZTNET][9])
* `GET /api/v1/org/:orgId/user` list org users ([ZTNET][10])

### `ztnet org --help`

```text
USAGE:
  ztnet org <SUBCOMMAND>

SUBCOMMANDS:
  list           List organizations you have access to
  get            Get organization details
  users          Manage/list users in an org
```

### `ztnet org list`

```text
USAGE:
  ztnet org list [--details | --ids-only]

OPTIONS:
  --details      Fetch org details (may perform N+1 calls)
  --ids-only     Print only org IDs (fast; default for --json scripts)
```

list: `GET /api/v1/org` ([ZTNET][6])

* With `--details`: for each id, `GET /api/v1/org/:orgId` ([ZTNET][9])

### `ztnet org get <ORG>`

```text
USAGE:
  ztnet org get <ORG>
```

API: `GET /api/v1/org/:orgId` ([ZTNET][9])

### `ztnet org users list`

```text
USAGE:
  ztnet org users list --org <ORG>
```

API: `GET /api/v1/org/:orgid/user` ([ZTNET][10])

---

## `network` commands (personal + org)

### Endpoints covered

Personal:

* `GET /api/v1/network` list networks ([ZTNET][5])
* `POST /api/v1/network` create network ([ZTNET][11])
* `GET /api/v1/network/:networkId` get network info ([ZTNET][12])
* `GET /api/v1/network/:networkId/member` list members ([ZTNET][13])
* `POST /api/v1/network/:networkId/member/:memberId` update member ([ZTNET][14])
* `DELETE /api/v1/network/:networkId/member/:memberId` stash/delete member ([ZTNET][15])

Organization:

* `GET /api/v1/org/:orgId/network` list org networks ([ZTNET][16])
* `POST /api/v1/org/:orgId/network` create org network ([ZTNET][17])
* `GET /api/v1/org/:orgId/network/:networkId` get org network info ([ZTNET][18])
* `POST /api/v1/org/:orgId/network/:networkId` update org network ([ZTNET][19])
* `GET /api/v1/org/:orgId/network/:networkId/member` list org network members ([ZTNET][20])
* `GET /api/v1/org/:orgId/network/:networkId/member/:memberId` get org member ([ZTNET][21])
* `POST /api/v1/org/:orgId/network/:networkId/member/:memberId` update org member ([ZTNET][22])
* `DELETE /api/v1/org/:orgId/network/:networkId/member/:memberId` stash/delete org member ([ZTNET][23])

All request bodies/fields below are aligned with your extracted inventory.

---

### `ztnet network --help`

```text
USAGE:
  ztnet network <SUBCOMMAND>

SUBCOMMANDS:
  list           List networks (personal by default; org if --org set)
  create         Create a network
  get            Get network details
  update         Update an org network (requires --org)
  member         Manage members within a network
```

---

### `ztnet network list`

```text
USAGE:
  ztnet network list [--org <ORG>] [--details | --ids-only] [--filter <EXPR>]

OPTIONS:
  --details      Fetch per-network details (N+1 calls)
  --ids-only     Print only network IDs (fast; best for scripts)
  --filter       Client-side filter (simple: name~=foo, private==true, etc.)
```

API mapping:

* Personal: `GET /api/v1/network` ([ZTNET][5])
* Org: `GET /api/v1/org/:orgId/network` ([ZTNET][16])

---

### `ztnet network create`

```text
USAGE:
  ztnet network create [--org <ORG>] [--name <NAME>]

OPTIONS:
  --name <NAME>  Optional; if omitted, server defaults apply
```

API mapping:

* Personal create: `POST /api/v1/network` ([ZTNET][11])
* Org create: `POST /api/v1/org/:orgId/network` ([ZTNET][17])

Request body: `{ name?: string }`

---

### `ztnet network get <NETWORK>`

```text
USAGE:
  ztnet network get <NETWORK> [--org <ORG>]
```

API mapping:

* Personal: `GET /api/v1/network/:networkId` ([ZTNET][12])
* Org: `GET /api/v1/org/:orgId/network/:networkId` ([ZTNET][18])

---

### `ztnet network update <NETWORK>` (org only)

ZTNet REST only exposes update for **org networks**, not personal networks (per your inventory).

```text
USAGE:
  ztnet network update <NETWORK> --org <ORG>
                      [--name <NAME>]
                      [--description <TEXT>]
                      [--mtu <MTU>]
                      [--private | --public]
                      [--flow-rule <TEXT> | --flow-rule-file <PATH>]
                      [--dns-domain <DOMAIN>]
                      [--dns-server <IP>...]
                      [--body <JSON> | --body-file <PATH>]

NOTES:
  - Any flags provided build a JSON body.
  - --body/--body-file overrides flag-built body.
```

API mapping:

* `POST /api/v1/org/:orgId/network/:networkId` ([ZTNET][19])

Supported body fields (from inventory): `name`, `description`, `flowRule`, `mtu`, `private`, `dns{domain,servers}`, `ipAssignmentPools`, `routes`, `v4AssignMode`, `v6AssignMode`.

---

## `network member` commands

### `ztnet network member --help`

```text
USAGE:
  ztnet network member <SUBCOMMAND>

SUBCOMMANDS:
  list           List members on a network
  get            Get member details (org uses GET endpoi:contentReference[oaicite:53]{index=53}list)
  update         Update member fields (authorize, rename, etc.)
  authorize      Convenience: set authorized=true
  deauthorize    Convenience: set authorized=false
  delete         Stash/delete member (confirmation unless --yes)
  stash          Alias for delete (matches ZTNet "Stashed Members" semantics)
```

---

### `ztnet network member list <NETWORK>`

```text
USAGE:
  ztnet network member list <NETWORK> [--org <ORG>]
                            [--authorized | --unauthorized]
                            [--name <SUBSTRING>]
                            [--id <NODEID>]

OPTIONS:
  --authorized     Show only authorized==true
  --unauthorized   Show only authorized==false
  --name           Client-side contains match on member name
  --id             Exact match on member id (node id)
```

API mapping:

* Personal: `GET /api/v1/network/:networkId/member` ([ZTNET][13])
* Org: `GET /api/v1/org/:orgId/network/:networkId/member` ([ZTNET][20])

---

### `ztnet network member get <NETWORK> <MEMBER>`

```text
USAGE:
  ztnet network member get <NETWORK> <MEMBER> [--org <ORG>]

BEHAVIOR:
  - Org scope: calls GET /org/:orgId/network/:networkId/member/:memberId
  - Personal:  calls list endpoint and selects matching memberId
```

API mapping (org):

* `GET /api/v1/org/:orgId/network/:networkId/member/:memberId` ([ZTNET][21])

API mapping (personal):

* selection from `GET /api/v1/network/:networkId/member` ([ZTNET][13])---

### `ztnet network member update <NETWORK> <MEMBER>`

```text
USAGE:
  ztnet network member update <NETWORK> <MEMBER> [--org <ORG>]
                             [--name <NAME>]
                             [--description <TEXT>]      (personal only)
                             [--authorized | --unauthorized]
                             [--body <JSON> | --body-file <PATH>]

NOTES:
  - For org members, description is not supported in REST inventory.
  - --body/--body-file overrides flag-built body.
```

API mapping:

* Personal update: `POST /api/v1/network/:ne:contentReference[oaicite:59]{index=59}rId` ([ZTNET][14])
* Org update: `POST /api/v1/org/:orgId/network/:networkId/member/:memberId` ([ZTNET][22])

Body fields (from inventory):

* Personal member update: `name?`, `description?`, `authorized?`
* Org member update: `name?`, `authorized?`

---

### `ztnet network member authorize|deauthorize`

```text
USAGE:
  ztnet network member authorize   <NETWORK> <MEMBER> [--org <ORG>]
  ztnet network member deauthorize <NETWORK> <MEMBER> [--org <ORG>]

BEHAVIOR:
  - Equivalent to: member update ... --authorized / --unauthorized
```

---

### `ztnet network member delete <NETWORK> <MEMBER>`

````text
USAGE:
  ztnet network member delete <NETWORK> <MEMBER> [--org <ORG>] [-y|--yes]

BEHAVIOR:
  - Shows a confirmation prompt unless --yes/-y is set.
  - Explains that ZTNet "deletes" by stashing (revokes auth + marks dele:contentReference[oaicite:64]{index=64}h” behavior is documented for delete endpoints. :contentReference[oaicite:65]{index=65}  
Global `--yes/-y` is borrowed from common CLI practice (e.g., Railway). :contentReference[oaicite:66]{index=66}  

API mapping:
- Personal delete: `DELETE /api/v1/network/:networkId/member/:memberId` :contentReference[oaicite:67]{index=67}  
- Org delete: `DELETE /api/v1/org/:orgId/network/:networkId/member/:memberId` :contentReference[oaicite:68]{index=68}  

---

## `member` shortcut alias

For ergonomics, `member` is a shortcut for `network member` using default context:

```text
ztnet member list <NETWORK> [--org <ORG>] ...
ztnet member update <NETWORK> <MEMBER> ...
...
````

This is purely a UX alias; it maps to the same commands above.

---

## `stats` commands (admin)

Endpoint: `GET /api/v1/stats` (admin required). ([ZTNET][24])

### `ztnet stats --help`

```text
USAGE:
  ztnet stats <SUBCOMMAND>

SUBCOMMANDS:
  get            Fetch ZTNet application statistics (admin)
```

### `ztnet stats get`

```text
USAGE:
  ztnet stats get [--json]
```

---

## `planet` commands (private root support)

ZTNet docs explicitly mention a convenient endpoint to download the planet file: `/api/planet`. ([ZTNET][25])

### `ztnet planet --help`

```text
USAGE:
  ztnet planet <SUBCOMMAND>

SUBCOMMANDS:
  download       Download planet file to disk (or stdout)
```

### `ztnet planet download`

```text
USAGE:
  ztnet planet download [--out <PATH>] [--stdout] [--force]

DEFAULTS:
  --out ./planet     (if neither --out nor --stdout is provided)
```

API mapping:

* `GET /api/planet` ([ZTNET][25])

---

## `export` commands (derived artifacts)

This group is “client-side value add” built on REST calls. The main one should be **hosts file generation**, because ZTNet already documents a `zt2hosts` script that does exactly this: it calls the ZTNet API, filters authorized members, and formats `hosts(5)` output. ([ZTNET][4])

### `ztnet export --help`

```text
USAGE:
  ztnet export <SUBCOMMAND>

SUBCOMMANDS:
  hosts          Generate a hosts(5)-style file from network members
```

### `ztnet export hosts`

```text
USAGE:
  ztnet export hosts <NETWORK> [--org <ORG>]
                    --zone <DOMAIN>
                    [--out <PATH>]
                    [--authorized-only] [--include-unauthorized]
                    [--format hosts|csv|json]

OPTIONS:
  --zone <DOMAIN>         DNS zone suffix (e.g. ztnet.example)
  --authorized-only       Default: only authorized members
  --include-unauthorized  Include unauthorized members too
  --out <PATH>            Write to file instead of stdout
```

Implementation mapping (REST):

* member list + network get are required (mirrors the published script behavior). ([ZTNET][4])

---

## `api` escape hatch (raw HTTP)

This guarantees the CLI can still use endpoints you haven’t wrapped with nice subcommands yet.

### `ztnet api --help`

````text
USAGE:
  ztnet api <SUBCOMMAND>

SUBCOMMANDS:
  request        Make an arbitrary HTTP request
  get            Shortcut for request GET
  post           Shortcut for request POST
  delete         Shortcut for request DE:contentReference[oaicite:74]{index=74}api request`
```text
USAGE:
  ztnet api request:contentReference[oaicite:75]{index=75}                [--body <JSON> | --body-file <PATH>]
                    [--header <K:V>...]
                    [--no-auth]
                    [--raw]

EXAMPLES:
  ztnet api get /api/v1/network
  ztnet api request POST /api/v1/network --body '{"name":"lab"}'
````

Rules:

* If `<PATH>` starts with `/api/v1`, auto-add auth header unless `--no-auth`.
* Respect `--host` and `--timeout`.

---

## `trpc` (experimental UI backend coverage)

Your inventory includes the tRPC endpoint `/api/trpc/[trpc]` and routers like `network`, `networkMember`, `admin`, `org`, etc.
These generally require **NextAuth session cookie auth**, not `x-ztnet-auth`.

So the UX should be an “advanced/experimental” surface:

### `ztnet trpc --help`

```text
USAGE:
  ztnet trpc <SUBCOMMAND>

SUBCOMMANDS:
  list           List routers and procedures (from inventory)
  call           Call router.procedure with JSON input
```

### `ztnet trpc call`

```text
USAGE:
  ztnet trpc call <router.procedure>
                 [--input <JSON> | --input-file <PATH>]
                 [--cookie <COOKIE>] [--cookie-file <PATH>]
                 [--json]

NOTES:
  - Requires a valid NextAuth session cookie for protected procedures.
  - This is mainly to unlock admin/UI-only features not exposed in REST.
```

#### Procedures exposed (from your inventory)

Routers/procedures supported by `trpc call` (verbatim from the inventory):

* `network`: getUserNetworks, getNetworkById, deleteNetwork, ipv6, enableIpv4AutoAssign, managedRoutes, easyIpAssignment
* `networkMember`: getAll, getMemberById, create, Update, Tags, UpdateDatabaseOnly, stash, delete, getMemberAnotations, removeMemberAnotations, bulkDeleteStashed
* `auth`: register, me, update, validateResetPasswordToken, passwordResetLink, changePasswordFromJwt, sendVerificationEmail, validateEmailVerificationToken, updateUserOptions, setZtApi, setLocalZt, getApiToken, addApiToken, deleteApiToken, deleteUserDevice
* `mfaAuth`: mfaValidateToken, mfaResetLink, mfaResetValidation, validateRecoveryToken
* `admin`: updateUser, deleteUser, createUser, getUser, getUsers, generateInviteLink, getInvitationLink, deleteInvitationLink, getControllerStats, getAllOptions, changeRole, updateGlobalOptions, getMailTemplates, setMail, setMailTemplates, getDefaultMailTemplate, sendTestMail, unlinkedNetwork, assignNetworkToUser, addUserGroup, getUserGroups, deleteUserGroup, assignUserGroup, getIdentity, getPlanet, makeWorld, resetWorld, createBackup, downloadBackup, listBackups, deleteBackup, restoreBackup, uploadBackup
* `settings`: getAllOptions, getPublicOptions, getAdminOptions
* `org`: createOrg, deleteOrg, updateMeta, getOrgIdbyUserid, getAllOrg, getOrgUserRoleById, getPlatformUsers, getOrgUsers, getOrgById, createOrgNetwork, changeUserRole, sendMessage, getMessages, markMessagesAsRead, getOrgNotifications, addUser, leave, getLogs, preValidateUserInvite, generateInviteLink, resendInvite, inviteUserByMail, deleteInvite, getInvites, transferNetworkOwnership, deleteOrgWebhooks, addOrgWebhooks, getOrgWebhooks, updateOrganizationSettings, getOrganizationSettings, updateOrganizationNotificationSettings, getOrganizationNotificationTemplate, getDefaultOrganizationNotificationTemplate, updateOrganizationNotificationTemplate, sendTestOrganizationNotification
* `public`: registrationAllowed, getWelcomeMessage

---

## `completion` (shell UX)

```text
USAGE:
  ztnet completion <SHELL>

ARGS:
  <SHELL>  bash|zsh|fish|powershell|elvish
```

---

## Output UX rules

### Default (“human”) output

* `list` commands: table view (id, name, authorized, …)
* `get` commands: pretty key/value view

### Machine output

* `--json` forces JSON output (global). ([Railway Docs][2])
* `--output json|yaml|raw` is an explicit alternative.

### Exit codes (spec)

* `0` success
* `2` argument/validation error
* `3` auth error (401/403)
* `4` not found (404)
* `5` conflict/validation (409/422)
* `6` rate limited (429) after retries exhausted (rate limit is documented). ([ZTNET][24])

---

## One-to-one mapping checklist (so we don’t miss anything)

Everything in the REST/Next routes inventory is accounted for:

* `/api/v1/user` → `ztnet user create` ([ZTNET][7])
* `/api/v1/network` (GET/POST) → `ztnet network list/create` ([ZTNET][5])
* `/api/v1/network/:networkId` → `ztnet network get` ([ZTNET][12])
* `/api/v1/network/:networkId/member` → `ztnet network member list` ([ZTNET][13])
* `/api/v1/network/:networkId/member/:memberId` (POST/DELETE) → `member update/delete` ([ZTNET][14])
* `/api/v1/stats` → `ztnet stats get` ([ZTNET][24])
* `/api/v1/org` + `/api/v1/org/:orgId` → `ztnet org list/get` ([ZTNET][6])
* `/api/v1/org/:orgId/user` → `ztnet org users list` ([ZTNET][10])api/v1/org/:orgId/network`(GET/POST) →`ztnet network list/create --org ...` ([ZTNET][16])
* `/api/v1/org/:orgId/network/:networkId` (GET/POST) → `ztnet network get/update --org ...` ([ZTNET][18])
* `/api/v1/org/:orgId/network/:networkId/member` → `member list --org ...` ([ZTNET][20])
* `/api/v1/org/:orgId/network/:networkId/member/:memberId` (GET/POST/DELETE) → `member get/update/delete --org ...` ([ZTNET][21])
* `/api/planet` → `ztnet planet download` ([ZTNET][25])
* tRPC + other Next routes → `ztnet trpc call` (experimental)

---

If you waan turn this UX spec into:

* a **Clap command module layout** (Rust types + subcommands + shared global options), and/or
* a **first-pass generated markdown CLI reference** that mirrors `--help` output exactly.



[1]: https://ztnet.network/Rest%20Api/Personal/Network/ztnet-network-rest-api "ZTNet Network Rest API | ZTNET - ZeroTier Web UI"
[2]: https://docs.railway.com/cli/global-options?utm_source=chatgpt.com "Global Options"
[3]: https://docs.sentry.io/cli/configuration/?utm_source=chatgpt.com "Configuration and Authentication - Sentry CLI"
[4]: https://ztnet.network/usage/create_dns_host "DNS Host File Generator | ZTNET - ZeroTier Web UI"
[5]: https://ztnet.network/Rest%20Api/Personal/Network/get-user-networks "Returns a list of Networks you have access to | ZTNET - ZeroTier Web UI"
[6]: https://ztnet.network/Rest%20Api/Organization/Organization/get-organization "Returns a list of Organizations you have access to. | ZTNET - ZeroTier Web UI"
[7]: https://ztnet.network/Rest%20Api/Personal/User/post-new-user "Create a new user | ZTNET - ZeroTier Web UI"
[8]: https://ztnet.network/Rest%20Api/Personal/User/post-new-user?utm_source=chatgpt.com "Create a new user | ZTNET - ZeroTier Web UI"
[9]: https://ztnet.network/Rest%20Api/Organization/Organization/get-organization-info "Returns information of the specified Organization. | ZTNET - ZeroTier Web UI"
[10]: https://ztnet.network/Rest%20Api/Organization/Users/get-organization-users?utm_source=chatgpt.com "Returns a list of Users in the organization"
[11]: https://ztnet.network/Rest%20Api/Personal/Network/create-new-network "Create New Network | ZTNET - ZeroTier Web UI"
[12]: https://ztnet.network/Rest%20Api/Personal/Network/get-network-info?utm_source=chatgpt.com "Returns information about a specific network"
[13]: https://ztnet.network/Rest%20Api/Personal/Network-Members/get-network-member-info?utm_source=chatgpt.com "Returns a list of Members on the network"
[14]: https://ztnet.network/Rest%20Api/Personal/Network-Members/modify-a-network-member?utm_source=chatgpt.com "Modify a network member | ZTNET - ZeroTier Web UI"
[15]: https://ztnet.network/Rest%20Api/Personal/Network-Members/delete-network-member?utm_source=chatgpt.com "Delete a network member | ZTNET - ZeroTier Web UI"
[16]: https://ztnet.network/Rest%20Api/Organization/Network/get-user-networks "Returns a list of Networks in the Organization you have access to | ZTNET - ZeroTier Web UI"
[17]: https://ztnet.network/Rest%20Api/Organization/Network/create-new-network "Create New Network within the Organization | ZTNET - ZeroTier Web UI"
[18]: https://ztnet.network/Rest%20Api/Organization/Network/get-network-info "Returns information about a specific organization network | ZTNET - ZeroTier Web UI"
[19]: https://ztnet.network/Rest%20Api/Organization/Network/update-network-info "Update a specific organization network | ZTNET - ZeroTier Web UI"
[20]: https://ztnet.network/Rest%20Api/Organization/Network-Members/get-network-member-info "Returns a list of Members in a organization network | ZTNET - ZeroTier Web UI"
[21]: https://ztnet.network/Rest%20Api/Organization/Network-Members/get-network-member-by-id-info "Returns a specific organization network member | ZTNET - ZeroTier Web UI"
[22]: https://ztnet.network/Rest%20Api/Organization/Network-Members/modify-a-organization-network-member "Modify a organization network member | ZTNET - ZeroTier Web UI"
[23]: https://ztnet.network/Rest%20Api/Organization/Network-Members/delete-network-member "Delete a organization network member | ZTNET - ZeroTier Web UI"
[24]: https://ztnet.network/Rest%20Api/Application/Statistics/ztnet-statistics-rest-api?utm_source=chatgpt.com "ZTNet Statistics Rest API | ZTNET - ZeroTier Web UI"
[25]: https://ztnet.network/usage/private_root "Private Root Servers with ZTNet | ZTNET - ZeroTier Web UI"
