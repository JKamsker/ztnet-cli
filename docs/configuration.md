# Configuration

ztnet-cli uses a TOML configuration file with named profiles. This document covers the file format, environment variables, and how configuration values are resolved.

## Config file location

| Platform | Default path |
|----------|-------------|
| Linux | `~/.config/ztnet/config.toml` or `$XDG_CONFIG_HOME/ztnet/config.toml` |
| macOS | `~/Library/Application Support/ztnet/config.toml` |
| Windows | `%APPDATA%\ztnet\config.toml` |

Print the path on your system:

```bash
ztnet config path
```

The config file is created automatically when you first run `auth set-token` or `config set`.

## File format

```toml
active_profile = "default"

[profiles.default]
host = "http://localhost:3000"
token = "your-api-token-here"
default_org = ""
default_network = ""
output = "table"
timeout = "30s"
retries = 3

[profiles.production]
host = "https://ztnet.example.com"
token = "prod-api-token"
output = "json"
timeout = "60s"
```

### Top-level keys

| Key | Type | Description |
|-----|------|-------------|
| `active_profile` | string | Name of the profile to use by default. Defaults to `"default"` if not set. |
| `profiles` | table | Map of profile names to their configuration. |

### Profile keys

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `host` | string | `http://localhost:3000` | Base URL of the ZTNet instance |
| `token` | string | _(none)_ | API token (`x-ztnet-auth` header value) |
| `default_org` | string | _(none)_ | Default organization ID or name for `--org` |
| `default_network` | string | _(none)_ | Default network ID or name for `--network` |
| `output` | string | `table` | Output format: `table`, `json`, `yaml`, `raw` |
| `timeout` | string | `30s` | HTTP timeout in [humantime](https://docs.rs/humantime) format (e.g., `30s`, `2m`, `1h`) |
| `retries` | integer | `3` | Number of retries for transient errors (5xx, timeouts, rate limits) |

## Environment variables

Environment variables override config file values but are overridden by CLI flags.

| Variable | Maps to | Description |
|----------|---------|-------------|
| `ZTNET_HOST` | `--host` | ZTNet base URL |
| `API_ADDRESS` | `--host` | Alternative (zt2hosts compat) |
| `ZTNET_TOKEN` | `--token` | API token |
| `ZTNET_API_TOKEN` | `--token` | Alternative (zt2hosts compat) |
| `ZTNET_PROFILE` | `--profile` | Profile name |
| `ZTNET_OUTPUT` | `--output` | Output format |

Example:

```bash
export ZTNET_HOST=https://ztnet.example.com
export ZTNET_TOKEN=sk_your_token
ztnet network list
```

## Precedence

Configuration values are resolved in this order (highest priority first):

1. **CLI flags** (`-H`, `-t`, `--org`, `--output`, etc.)
2. **Environment variables** (`ZTNET_HOST`, `ZTNET_TOKEN`, etc.)
3. **Config file** (active profile in `config.toml`)
4. **Hardcoded defaults** (`http://localhost:3000`, `table` output, `30s` timeout, 3 retries)

## Managing profiles

### Create / switch profiles

```bash
# Save a token to a specific profile
ztnet --profile staging auth set-token STAGING_TOKEN
ztnet config set profiles.staging.host https://staging.example.com

# Switch the active profile
ztnet auth profiles use staging

# List all profiles
ztnet auth profiles list
```

### View current config

```bash
# Show the effective auth context (host, profile, token, org, network)
ztnet auth show

# Show the full merged configuration
ztnet config list
```

### Set context defaults

Set a default org and/or network so you don't need `--org` and `--network` on every command:

```bash
# Set defaults
ztnet config context set --org my-org --network my-network

# Show current context
ztnet config context show

# Clear defaults
ztnet config context clear
```

## Config commands

| Command | Description |
|---------|-------------|
| `config path` | Print the config file path |
| `config get <KEY>` | Get a config value (e.g., `profiles.default.host`) |
| `config set <KEY> <VALUE>` | Set a config value |
| `config unset <KEY>` | Remove a config value |
| `config list` | Print the full effective config (tokens redacted) |
| `config context show` | Show default org/network for the active profile |
| `config context set` | Set default org and/or network |
| `config context clear` | Clear default org and network |

### Dotted key examples

```bash
ztnet config get active_profile
ztnet config set profiles.default.host https://ztnet.example.com
ztnet config set profiles.default.output json
ztnet config set profiles.default.timeout 60s
ztnet config set profiles.default.retries 5
ztnet config unset profiles.default.default_org
```
