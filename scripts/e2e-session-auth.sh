#!/usr/bin/env bash
set -euo pipefail

compose_file="${ZTNET_E2E_COMPOSE_FILE:-e2e/compose.yml}"
base_url="${ZTNET_E2E_BASE_URL:-http://localhost:3000}"
profile="${ZTNET_E2E_PROFILE:-e2e}"

email="${ZTNET_E2E_EMAIL:-admin@local.test}"
password="${ZTNET_E2E_PASSWORD:-TestPassword123!}"
name="${ZTNET_E2E_NAME:-Admin}"

cleanup() {
  docker compose -f "$compose_file" down -v >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker compose -f "$compose_file" up -d

echo "Waiting for ZTNet at $base_url ..."
for _ in $(seq 1 180); do
  code="$(curl -s -o /dev/null -w "%{http_code}" "$base_url" || true)"
  if [[ "$code" =~ ^2|^3|^4 ]]; then
    break
  fi
  sleep 1
done

cargo build
bin="target/debug/ztnet"

"$bin" --profile "$profile" config set host "$base_url" --no-validate >/dev/null

echo "Bootstrapping first user (no-auth) ..."
"$bin" --profile "$profile" user create --email "$email" --password "$password" --name "$name" --no-auth >/dev/null

echo "Logging in via session auth ..."
"$bin" --profile "$profile" auth login --email "$email" --password "$password" >/dev/null

echo "Validating admin tRPC access ..."
users_json="$("$bin" --profile "$profile" --output json admin users list)"
echo "$users_json" | jq -e 'type == "array" and length > 0' >/dev/null

echo "OK"

