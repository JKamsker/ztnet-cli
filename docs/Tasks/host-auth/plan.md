# Host-bound auth + per-host defaults (ZTNet CLI)

## Summary
Credentials (API token + session cookies) must be **bound to a host** so we never accidentally send Host A’s credentials to Host B. Multiple credentials per host remain possible via multiple named profiles. A new `host_defaults` mapping selects the default profile for a given host when users target a host explicitly.

## Goals
- Host-bound credential usage:
  - Stored profile token/session cookies are used **only** when the profile host matches the request host.
  - If the user targets another host, creds for that host are selected (implicitly or explicitly).
- Multiple credentials per host:
  - Keep named `profiles` as credential containers.
  - Add per-host “default profile” selection via `host_defaults`.
- Safe credential storage:
  - `ztnet auth set-token` and `ztnet auth login` require a host from `--host`, env, or the target profile’s configured host.
  - Do not silently bind to the hardcoded fallback host (`http://localhost:3000`) unless it was explicitly configured/provided.

## Non-goals
- Replacing `profiles` with a new `[hosts]` store (keep the current config structure).
- Making environment-provided tokens host-bound (env/flags are considered explicit overrides).

## Config changes (backward compatible)
Add top-level:

```toml
[host_defaults]
"https://ztnet.example.com" = "prod"
"https://staging.ztnet.example.com" = "staging"
```

Keys are canonical host keys (see below). Values are profile names.

## Canonical host key
Canonicalize any base URL to:
- `scheme://hostname` (hostname lowercased)
- include `:port` only if non-default (`http:80`, `https:443` omitted)
- ignore path/query/fragment, ignore trailing `/`
- IPv6 hosts formatted as `scheme://[addr](:port)?`

Examples:
- `https://ztnet.example.com/` → `https://ztnet.example.com`
- `https://ztnet.example.com:443` → `https://ztnet.example.com`
- `http://localhost:3000` → `http://localhost:3000`

## New commands: `auth hosts`
Add:
- `ztnet auth hosts list`
  - show each host (derived from `host_defaults` + `profiles.*.host`) and:
    - the default profile for that host (if any)
    - which profiles match that host
- `ztnet auth hosts set-default <HOST> [PROFILE]`
  - if `PROFILE` omitted:
    - if exactly 1 profile matches `<HOST>` → use it
    - if multiple profiles match `<HOST>` → pick lexicographically first profile name (deterministic “first profile”)
    - if no profiles match `<HOST>` → create a new inferred profile name (slugify), set its host, and use it
  - set `host_defaults[host_key] = profile`
- `ztnet auth hosts unset-default <HOST>`

### Inferred profile naming (slugify)
When creating a profile automatically:
- base name: slugify hostname (append `-<port>` if non-default)
- lowercase; replace non `[a-z0-9]` with `-`; collapse repeats; trim `-`
- ensure uniqueness by suffixing `-2`, `-3`, …

Examples:
- `https://ztnet.example.com` → `ztnet-example-com`
- `http://localhost:3000` → `localhost-3000`

## Runtime resolution rules (host-bound)
- If user provides `--host` (or env host), that is the target host.
  - If user explicitly provides `--profile` (or env profile) and that profile has a host set that doesn’t match: **error**.
  - Else select profile by:
    1) `host_defaults[target_host]` if present
    2) else profiles whose `host` matches (1 match → that; many → lexicographically first)
    3) else no matching profile → proceed with no stored creds (token/cookies = None)
- If user does not provide host:
  - use explicit profile else `active_profile` else `"default"`
  - host comes from that profile (or falls back to `http://localhost:3000` for non-auth commands)

Stored token/cookies are included only when profile host matches the target host.

## Credential storage rules (`auth set-token` / `auth login`)
- Must have a host from:
  - `--host`, or
  - env (`ZTNET_HOST` / `API_ADDRESS`), or
  - target profile’s configured host
- If profile host missing and a host is available → set profile host (binding).
- If host default is missing for that host → set `host_defaults[host_key] = profile` (do not overwrite existing mapping).

## Acceptance criteria
- `ztnet -H https://other ...` never uses the token/session from the active profile if it points at a different host.
- Multiple profiles for the same host are supported; host default controls implicit selection.
- `ztnet auth set-token TOKEN` errors if no host is configured/provided.
- `ztnet auth login ...` errors if no host is configured/provided.

