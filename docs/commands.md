# Command Reference

Complete reference for every ztnet-cli command, subcommand, and flag.

## Global options

These flags are available on all commands:

```
-H, --host <URL>          ZTNet base URL (e.g., https://ztnet.example.com)
-t, --token <TOKEN>       API token (x-ztnet-auth header)
    --profile <NAME>      Named config profile (default: "default")
    --org <ORG>           Organization scope (ID or resolvable name)
    --network <NETWORK>   Default network (ID or resolvable name)
    --json                Output as JSON (shortcut for --output json)
-o, --output <FORMAT>     Output format: table|json|yaml|raw (default: table)
    --no-color            Disable ANSI colors
    --quiet               Suppress interactive output (prompts, spinners)
-v, --verbose             Verbose logging (repeat for more: -vv, -vvv)
    --timeout <DURATION>  HTTP timeout (default: 30s, humantime format)
    --retries <N>         Retry count for transient errors (default: 3)
    --dry-run             Print the HTTP request and exit without sending it
-y, --yes                 Skip confirmation prompts
-h, --help                Print help
-V, --version             Print version
```

---

## auth

Manage API tokens, profiles, and connectivity.

### auth set-token

Save an API token to a config profile.

```bash
ztnet auth set-token <TOKEN>
ztnet auth set-token --stdin         # read token from stdin
ztnet --profile prod auth set-token <TOKEN>
```

| Flag | Description |
|------|-------------|
| `--stdin` | Read the token from standard input instead of an argument |
| `--profile <NAME>` | Profile to store the token under (global flag) |

### auth unset-token

Remove the saved token from a profile.

```bash
ztnet auth unset-token
ztnet --profile prod auth unset-token
```

### auth show

Display the effective auth context: host, profile, token (redacted), org, and network.

```bash
ztnet auth show
```

### auth test

Validate your token by making an API call.

```bash
ztnet auth test
ztnet auth test --org my-org    # test org-scoped access
```

### auth profiles list

Show all profiles and which one is active.

```bash
ztnet auth profiles list
```

### auth profiles use

Switch the active profile.

```bash
ztnet auth profiles use production
```

---

## config

View and edit the config file, manage defaults.

### config path

Print the config file location.

```bash
ztnet config path
```

### config get

Read a config value using dotted key notation.

```bash
ztnet config get active_profile
ztnet config get profiles.default.host
```

### config set

Write a config value.

```bash
ztnet config set profiles.default.host https://ztnet.example.com
ztnet config set profiles.default.output json
```

### config unset

Remove a config value.

```bash
ztnet config unset profiles.default.default_org
```

### config list

Print the full effective config with tokens redacted.

```bash
ztnet config list
```

### config context show

Display the default org and network for the active profile.

```bash
ztnet config context show
```

### config context set

Set default org and/or network so you don't have to pass `--org` / `--network` every time.

```bash
ztnet config context set --org my-org
ztnet config context set --network my-net
ztnet config context set --org my-org --network my-net
```

### config context clear

Remove default org and network from the active profile.

```bash
ztnet config context clear
```

---

## user

Create platform users. Primarily used for bootstrapping the first admin user.

### user create

```bash
ztnet user create \
  --email admin@example.com \
  --password SecurePass123 \
  --name "Admin"
```

| Flag | Description |
|------|-------------|
| `--email <EMAIL>` | **(required)** User email |
| `--password <PASSWORD>` | **(required)** User password |
| `--name <NAME>` | **(required)** Display name |
| `--expires-at <ISO8601>` | Token expiry timestamp |
| `--generate-api-token` | Ask the server to generate an API token |
| `--store-token` | Save the returned token to the config profile |
| `--print-token` | Print the returned token to stdout |
| `--no-auth` | Skip the `x-ztnet-auth` header (required for bootstrapping the first user on an empty database) |

**Bootstrap example** (fresh ZTNet, no existing users):

```bash
ztnet user create \
  --email admin@example.com \
  --password SecurePass123 \
  --name "Admin" \
  --generate-api-token \
  --store-token \
  --no-auth
```

---

## org

List and inspect organizations.

### org list

```bash
ztnet org list
ztnet org list --details      # fetch full details per org (N+1 calls)
ztnet org list --ids-only     # print only org IDs
```

### org get

```bash
ztnet org get <ORG>           # by ID or name
```

### org users list

```bash
ztnet org users list --org my-org
```

---

## network

Create, list, get, and update networks.

### network list

```bash
ztnet network list
ztnet network list --org my-org         # org-scoped
ztnet network list --details            # fetch full details (N+1 calls)
ztnet network list --ids-only           # print only network IDs
ztnet network list --filter "name~=dev" # filter by name substring
```

| Flag | Description |
|------|-------------|
| `--org <ORG>` | List networks in this organization |
| `--details` | Fetch per-network details (additional API calls) |
| `--ids-only` | Print only the network IDs |
| `--filter <EXPR>` | Client-side filter expression (see below) |

**Filter syntax:**

Combine filters with commas:

```
name~=substring       case-insensitive substring match on network name
private==true         exact match on the private flag
private==false        exact match on the private flag
```

Example: `--filter "name~=prod,private==true"`

### network create

```bash
ztnet network create --name "my-network"
ztnet network create --org my-org --name "team-network"
```

### network get

```bash
ztnet network get <NETWORK>             # by ID or name
ztnet network get <NETWORK> --org my-org
```

### network update

Update an organization-scoped network. (Personal network updates are not exposed in the ZTNet API.)

```bash
ztnet network update <NETWORK> --org my-org [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Rename the network |
| `--description <TEXT>` | Set the description |
| `--mtu <MTU>` | Set the MTU |
| `--private` | Make the network private |
| `--public` | Make the network public |
| `--flow-rule <TEXT>` | Set ZeroTier flow rules inline |
| `--flow-rule-file <PATH>` | Set flow rules from a file |
| `--dns-domain <DOMAIN>` | Set the DNS search domain |
| `--dns-server <IP>` | Add a DNS server (repeatable) |
| `--body <JSON>` | Override request body with raw JSON |
| `--body-file <PATH>` | Read request body from file |

---

## member / network member

Manage network members. `member` is a top-level alias for `network member`.

### member list

```bash
ztnet member list <NETWORK>
ztnet member list <NETWORK> --org my-org
ztnet member list <NETWORK> --authorized      # only authorized members
ztnet member list <NETWORK> --unauthorized    # only unauthorized members
ztnet member list <NETWORK> --name "alice"    # filter by name substring
ztnet member list <NETWORK> --id abc123       # filter by node ID
```

### member get

```bash
ztnet member get <NETWORK> <MEMBER>
ztnet member get <NETWORK> <MEMBER> --org my-org
```

### member update

```bash
ztnet member update <NETWORK> <MEMBER> [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Set the member name |
| `--description <TEXT>` | Set the member description (personal scope only) |
| `--authorized` | Authorize the member |
| `--unauthorized` | Deauthorize the member |
| `--body <JSON>` | Override with raw JSON |
| `--body-file <PATH>` | Read body from file |

### member authorize

Convenience shortcut to authorize a member.

```bash
ztnet member authorize <NETWORK> <MEMBER>
ztnet member authorize <NETWORK> <MEMBER> --org my-org
```

### member deauthorize

Convenience shortcut to deauthorize a member.

```bash
ztnet member deauthorize <NETWORK> <MEMBER>
```

### member delete

Stash (soft-delete) a member. Prompts for confirmation unless `-y` is passed.

```bash
ztnet member delete <NETWORK> <MEMBER>
ztnet member delete <NETWORK> <MEMBER> -y    # skip confirmation
```

Alias: `member stash`

---

## stats

### stats get

Fetch admin-level statistics from ZTNet.

```bash
ztnet stats get
ztnet stats get --json
```

---

## planet

### planet download

Download a custom planet file from ZTNet.

```bash
ztnet planet download                     # writes to ./planet
ztnet planet download --out /etc/planet   # custom path
ztnet planet download --stdout            # write to stdout
ztnet planet download --force             # overwrite existing file
```

---

## export

Generate derived files from network data.

### export hosts

Generate a hosts(5) file, CSV, or JSON from network members.

```bash
ztnet export hosts <NETWORK> --zone ztnet.local
ztnet export hosts <NETWORK> --zone ztnet.local --out /tmp/hosts
ztnet export hosts <NETWORK> --zone ztnet.local --format csv
ztnet export hosts <NETWORK> --zone ztnet.local --format json
```

| Flag | Description |
|------|-------------|
| `--zone <DOMAIN>` | **(required)** DNS zone suffix (e.g., `ztnet.local`) |
| `--out <PATH>` | Write to file instead of stdout |
| `--format <FMT>` | Output format: `hosts` (default), `csv`, `json` |
| `--authorized-only` | Include only authorized members (default) |
| `--include-unauthorized` | Include unauthorized members too |
| `--org <ORG>` | Organization scope |

---

## api

Raw HTTP escape hatch for calling any ZTNet endpoint.

### api request

```bash
ztnet api request GET /api/v1/network
ztnet api request POST /api/v1/network --body '{"name":"test"}'
ztnet api request POST /api/v1/network --body-file payload.json
ztnet api request GET /api/v1/network --header "X-Custom: value"
ztnet api request GET /api/v1/network --no-auth
ztnet api request GET /api/planet --raw
```

| Flag | Description |
|------|-------------|
| `--body <JSON>` | Request body |
| `--body-file <PATH>` | Read body from file |
| `--header <K:V>` | Add a custom header (repeatable) |
| `--no-auth` | Skip the `x-ztnet-auth` header |
| `--raw` | Output raw bytes instead of JSON |

### api get / api post / api delete

Convenience shortcuts:

```bash
ztnet api get /api/v1/network
ztnet api post /api/v1/network --body '{"name":"test"}'
ztnet api delete /api/v1/network/abc123
```

---

## trpc

Call tRPC procedures on the ZTNet backend. This is experimental and requires a NextAuth session cookie (not an API token).

### trpc list

List all known routers and procedures.

```bash
ztnet trpc list
```

### trpc call

```bash
ztnet trpc call network.getAll --cookie "next-auth.session-token=..."
ztnet trpc call network.getAll --cookie-file cookie.txt
ztnet trpc call networkMember.create --input '{"networkId":"abc123"}' --cookie "..."
```

| Flag | Description |
|------|-------------|
| `--input <JSON>` | JSON input for the procedure |
| `--input-file <PATH>` | Read input from file |
| `--cookie <COOKIE>` | NextAuth session cookie |
| `--cookie-file <PATH>` | Read cookie from file |

---

## completion

Generate shell completions.

```bash
ztnet completion bash
ztnet completion zsh
ztnet completion fish
ztnet completion powershell
ztnet completion elvish
```

**Installation:**

```bash
# Bash
ztnet completion bash > ~/.local/share/bash-completion/completions/ztnet

# Zsh (add ~/.zfunc to your fpath)
ztnet completion zsh > ~/.zfunc/_ztnet

# Fish
ztnet completion fish > ~/.config/fish/completions/ztnet.fish

# PowerShell
ztnet completion powershell >> $PROFILE
```
