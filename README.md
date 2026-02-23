# ztnet-cli

> A fast, ergonomic command-line interface for managing [ZeroTier](https://www.zerotier.com/) networks through [ZTNet](https://github.com/sinamics/ztnet).

```
$ ztnet network list --json
[
  { "nwid": "abcdef1234567890", "name": "my-network", "private": true }
]
```

## Features

- **Full REST API coverage** &mdash; networks, members, orgs, stats, planet files
- **Named profiles** &mdash; switch between multiple ZTNet instances with `auth profiles use`
- **Host-bound credentials** &mdash; stored tokens/sessions are only used for their configured host
- **Smart name resolution** &mdash; reference networks and orgs by name, not just ID
- **Flexible output** &mdash; table, JSON, YAML, or raw for scripting
- **Hosts file export** &mdash; generate `/etc/hosts` entries from network members
- **Raw API escape hatch** &mdash; call any endpoint with `api get /api/v1/...`
- **Dry-run mode** &mdash; preview HTTP requests without sending them
- **Automatic retries** &mdash; exponential backoff on transient errors and rate limits
- **Shell completions** &mdash; bash, zsh, fish, PowerShell, elvish

## Quickstart

### Install

<details>
<summary>cargo install</summary>

```bash
cargo install ztnet
```

</details>

<details>
<summary>npm (prebuilt binaries)</summary>

```bash
npm install -g ztnet-cli
```

</details>

<details>
<summary>winget (Windows)</summary>

```powershell
winget install ztnet
```

</details>

<details>
<summary>Build from source</summary>

```bash
git clone https://github.com/JKamsker/ztnet-cli.git
cd ztnet-cli
cargo build --release
# Binary: target/release/ztnet  (target/release/ztnet.exe on Windows)
```

</details>

### Authenticate

```bash
# Point to your ZTNet instance (validated automatically)
ztnet config set host https://ztnet.example.com

# Save your API token (grab it from ZTNet web UI -> Account -> API tokens)
ztnet auth set-token YOUR_API_TOKEN

# Or read from stdin to keep it out of shell history
echo "YOUR_API_TOKEN" | ztnet auth set-token --stdin

# Verify it works
ztnet auth test
```

### Core commands

```bash
# List all your networks
ztnet network list

# Create a new network
ztnet network create --name "dev-network"

# List members of a network (by name!)
ztnet member list dev-network

# Authorize a member
ztnet member authorize dev-network abc1234567

# Export hosts file for DNS
ztnet export hosts dev-network --zone ztnet.local

# Get output as JSON for scripting
ztnet network list --json
```

## Command overview

| Command | Description |
|---------|-------------|
| `auth` | Manage API tokens, profiles, and test connectivity |
| `config` | View and edit configuration, set defaults |
| `user` | Create platform users (admin/bootstrap) |
| `org` | List and inspect organizations |
| `network` | Create, list, get, and update networks |
| `member` | List, authorize, deauthorize, and manage members |
| `stats` | Fetch admin statistics |
| `planet` | Download custom planet files |
| `export` | Generate hosts files, CSV, or JSON from members |
| `api` | Raw HTTP requests to any endpoint |
| `trpc` | Call tRPC procedures (experimental) |
| `completion` | Generate shell completion scripts |

## Global flags

```
-H, --host <URL>        ZTNet base URL
-t, --token <TOKEN>     API token
--profile <NAME>        Config profile to use (default: "default")
--org <ORG>             Organization scope (ID or name)
--network <NETWORK>     Default network (ID or name)
-o, --output <FMT>      Output format: table, json, yaml, raw
--json                  Shortcut for --output json
--dry-run               Print the HTTP request without sending it
--timeout <DURATION>    HTTP timeout (default: 30s)
--retries <N>           Retry count for transient errors (default: 3)
-y, --yes               Skip confirmation prompts
-v, --verbose           Increase log verbosity
--no-color              Disable ANSI colors
--quiet                 Suppress interactive output
```

## Configuration

Config lives in a TOML file with named profiles:

| Platform | Path |
|----------|------|
| Linux | `~/.config/ztnet/config.toml` |
| macOS | `~/Library/Application Support/ztnet/config.toml` |
| Windows | `%APPDATA%\ztnet\config.toml` |

```toml
active_profile = "default"

[profiles.default]
host = "https://ztnet.example.com"
token = "your-api-token"
output = "table"

[profiles.staging]
host = "https://staging.ztnet.example.com"
token = "staging-token"
```

**Precedence:** CLI flags > environment variables > config file > defaults.

See [docs/configuration.md](docs/configuration.md) for the full reference.

## Documentation

| Document | Description |
|----------|-------------|
| [Configuration](docs/configuration.md) | Profiles, environment variables, config file format, precedence rules |
| [Command Reference](docs/commands.md) | Every command, subcommand, flag, and option |
| [API Reference](docs/api-reference.md) | Endpoint mapping, authentication, HTTP client behavior, exit codes |
| [Development](docs/development.md) | Building from source, Docker setup, smoke tests, architecture |

## Shell completions

```bash
# Bash
ztnet completion bash > ~/.local/share/bash-completion/completions/ztnet

# Zsh
ztnet completion zsh > ~/.zfunc/_ztnet

# Fish
ztnet completion fish > ~/.config/fish/completions/ztnet.fish

# PowerShell
ztnet completion powershell >> $PROFILE
```

## Examples

### Multi-profile workflow

```bash
# Set up profiles for different environments
ztnet --profile prod config set host https://ztnet.prod.example.com
ztnet --profile prod auth set-token PROD_TOKEN

ztnet --profile staging config set host https://ztnet.staging.example.com
ztnet --profile staging auth set-token STAGING_TOKEN

# Optionally set defaults per host (used when you pass --host without --profile)
ztnet auth hosts set-default https://ztnet.prod.example.com prod
ztnet auth hosts set-default https://ztnet.staging.example.com staging

# Switch between them
ztnet auth profiles use prod
ztnet network list

ztnet auth profiles use staging
ztnet network list
```

### Organization-scoped operations

```bash
# Set a default org so you don't have to pass --org every time
ztnet config context set --org my-org

# Now all commands use that org
ztnet network list
ztnet network create --name "team-network"
ztnet member list team-network
```

### Scripting with JSON

```bash
# Get all network IDs
ztnet network list --ids-only

# Pipe member data into jq
ztnet member list my-network --json | jq '.[].name'

# Authorize all unauthorized members
for id in $(ztnet member list my-net --unauthorized --json | jq -r '.[].id'); do
  ztnet member authorize my-net "$id"
done
```

### Export hosts file

```bash
# Generate /etc/hosts entries
ztnet export hosts my-network --zone ztnet.local > /tmp/ztnet-hosts

# Or as JSON for further processing
ztnet export hosts my-network --zone ztnet.local --format json
```

### Dry-run and debugging

```bash
# See exactly what HTTP request would be made
ztnet --dry-run network list

# GET http://localhost:3000/api/v1/network
# x-ztnet-auth: sk_1…abcd
```

## License

AGPL-3.0-only (see `LICENSE`)
