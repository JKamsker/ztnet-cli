# API Reference

This document describes how ztnet-cli maps to ZTNet REST API endpoints, how authentication works, and the behavior of the built-in HTTP client.

## Authentication

All API requests include an `x-ztnet-auth` header with your API token:

```
x-ztnet-auth: your-api-token
```

The token is resolved from (highest priority first):

1. `--token` CLI flag
2. `ZTNET_TOKEN` or `ZTNET_API_TOKEN` environment variable
3. `token` field in a config profile whose configured host matches the target host

Profile selection is host-aware:
- If `--profile` is set, that profile is used (and if `--host` is also set, the profile host must match).
- If `--host` is set without `--profile`, the CLI selects the per-host default profile from `host_defaults` (or the first matching profile by name).

This prevents accidentally sending a token configured for Host A to Host B.

**Exception:** The `--no-auth` flag skips the auth header entirely. This is required for `user create` when bootstrapping the first user on an empty database, and available on `api request` for unauthenticated endpoints.

## Endpoint mapping

### Personal scope

These endpoints are used when `--org` is **not** specified.

| CLI command | Method | Endpoint |
|-------------|--------|----------|
| `network list` | GET | `/api/v1/network` |
| `network create` | POST | `/api/v1/network` |
| `network get <NW>` | GET | `/api/v1/network/{networkId}` |
| `member list <NW>` | GET | `/api/v1/network/{networkId}/member` |
| `member get <NW> <M>` | GET | `/api/v1/network/{networkId}/member/{memberId}` |
| `member update <NW> <M>` | POST | `/api/v1/network/{networkId}/member/{memberId}` |
| `member delete <NW> <M>` | DELETE | `/api/v1/network/{networkId}/member/{memberId}` |

### Organization scope

These endpoints are used when `--org` is specified (via flag, env, or config context).

| CLI command | Method | Endpoint |
|-------------|--------|----------|
| `org list` | GET | `/api/v1/org` |
| `org get <ORG>` | GET | `/api/v1/org/{orgId}` |
| `org users list` | GET | `/api/v1/org/{orgId}/user` |
| `network list --org` | GET | `/api/v1/org/{orgId}/network` |
| `network create --org` | POST | `/api/v1/org/{orgId}/network` |
| `network get --org` | GET | `/api/v1/org/{orgId}/network/{networkId}` |
| `network update --org` | POST | `/api/v1/org/{orgId}/network/{networkId}` |
| `member list --org` | GET | `/api/v1/org/{orgId}/network/{networkId}/member` |
| `member get --org` | GET | `/api/v1/org/{orgId}/network/{networkId}/member/{memberId}` |
| `member update --org` | POST | `/api/v1/org/{orgId}/network/{networkId}/member/{memberId}` |
| `member delete --org` | DELETE | `/api/v1/org/{orgId}/network/{networkId}/member/{memberId}` |

### Admin / other

| CLI command | Method | Endpoint |
|-------------|--------|----------|
| `user create` | POST | `/api/v1/user` |
| `stats get` | GET | `/api/v1/stats` |
| `planet download` | GET | `/api/planet` |

### tRPC (experimental)

| CLI command | Method | Endpoint |
|-------------|--------|----------|
| `trpc call <proc>` | POST | `/api/trpc/{procedure}?batch=1` |

tRPC uses NextAuth session cookies instead of the `x-ztnet-auth` token.

## Name resolution

When you pass a network or org by name instead of ID, ztnet-cli resolves it:

1. Fetches the list (e.g., `GET /api/v1/network`)
2. Filters by name (case-insensitive substring match)
3. If exactly one match is found, uses its ID
4. If zero or multiple matches are found, returns an error with details

This means `ztnet network get my-net` works as long as the name uniquely identifies a network.

## HTTP client behavior

### Retries

Transient errors are retried automatically with exponential backoff:

- **Retried:** 5xx status codes, 429 (rate limited), timeouts, connection errors
- **Not retried:** 4xx status codes (except 429), JSON parse errors, other client errors
- **Backoff:** starts at 200ms, doubles each retry, capped at 5s
- **Default retries:** 3 (configurable with `--retries`)

### Rate limiting

When a `429 Too Many Requests` response is received:

1. The `Retry-After` header is parsed (if present) to determine the wait time
2. If no `Retry-After` header, the standard backoff is used
3. If retries are exhausted, the CLI exits with code 6

### Timeouts

- Default: 30 seconds per request
- Configurable: `--timeout 60s` or `config set profiles.default.timeout 60s`
- Accepts [humantime](https://docs.rs/humantime) format: `30s`, `2m`, `1h30m`

### Dry-run mode

`--dry-run` prints the HTTP request that would be sent and exits without making any network calls:

```
GET http://localhost:3000/api/v1/network
x-ztnet-auth: sk_1…abcd

{
  "name": "test"
}
```

Tokens are redacted in dry-run output (first 4 and last 4 characters shown).

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success (also used for `--dry-run`) |
| 1 | General error (unexpected HTTP status, I/O error, JSON parse error) |
| 2 | Argument or validation error (missing config, invalid argument) |
| 3 | Authentication error (HTTP 401 or 403) |
| 4 | Not found (HTTP 404) |
| 5 | Conflict or validation error (HTTP 409 or 422) |
| 6 | Rate limited (HTTP 429 after retries exhausted) |

Use exit codes in scripts:

```bash
ztnet auth test
case $? in
  0) echo "authenticated" ;;
  3) echo "bad token" ;;
  *) echo "error" ;;
esac
```

## Output formats

All commands support multiple output formats via `--output` or `--json`:

| Format | Flag | Description |
|--------|------|-------------|
| `table` | `--output table` | Pretty ASCII tables (default, human-readable) |
| `json` | `--json` or `--output json` | Pretty-printed JSON |
| `yaml` | `--output yaml` | YAML |
| `raw` | `--output raw` | Compact single-line JSON (for piping) |

The `--quiet` flag suppresses interactive elements (confirmation prompts, spinners) while still printing the data output. Combine with `--json` for fully machine-readable output.
