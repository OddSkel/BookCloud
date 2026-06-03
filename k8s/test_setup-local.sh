#!/usr/bin/env bash
# test_setup.sh — run after startup_service.sh to prepare port-forwards and tokens.
# Usage: source ./test_setup.sh   <-- MUST be sourced, not executed directly

NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
KEYCLOAK_URL="http://localhost:8090"
KONG_URL="http://localhost:9000"
REALM="bookcloud"

# ── Kill any stale port-forwards on these ports (never fail if none exist) ────
echo "Cleaning up stale port-forwards..."
fuser -k 8090/tcp 2>/dev/null || true
fuser -k 9000/tcp 2>/dev/null || true
sleep 1

# ── Start port-forwards in background, output suppressed ─────────────────────
echo "Starting port-forwards..."
kubectl -n "$NAMESPACE" port-forward svc/keycloak 8090:80 >/dev/null 2>&1 &
PF_KEYCLOAK=$!
kubectl -n "$NAMESPACE" port-forward svc/kong 9000:80 >/dev/null 2>&1 &
PF_KONG=$!

# ── Wait for Keycloak to respond ──────────────────────────────────────────────
echo "Waiting for Keycloak..."
attempts=0
until curl -sf "$KEYCLOAK_URL/realms/master" >/dev/null 2>&1; do
  sleep 2
  (( attempts++ )) || true
  if (( attempts > 20 )); then
    echo "Erro: Keycloak did not respond in time." >&2
    return 1
  fi
done
echo "✓ Keycloak ready."

# ── Resolve client secret from Keycloak admin API ────────────────────────────
KEYCLOAK_ADMIN="${BOOKCLOUD_KEYCLOAK_ADMIN:-admin}"
KEYCLOAK_ADMIN_PASSWORD="${BOOKCLOUD_KEYCLOAK_PASSWORD:-bookcloud-pass}"

ADMIN_TOKEN=$(curl -s -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
  -d "client_id=admin-cli&grant_type=password&username=$KEYCLOAK_ADMIN&password=$KEYCLOAK_ADMIN_PASSWORD" \
  | jq -r '.access_token')

CLIENT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/clients?clientId=bookcloud-app" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.[0].id')

CLIENT_SECRET=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/clients/$CLIENT_UUID/client-secret" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq -r '.value')

if [ -z "$CLIENT_SECRET" ] || [ "$CLIENT_SECRET" = "null" ]; then
  echo "Erro: could not resolve client secret from Keycloak." >&2
  return 1
fi
echo "✓ Client secret resolved."

# ── Fetch tokens ──────────────────────────────────────────────────────────────
_token() {
  curl -s -X POST "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/token" \
    -d "grant_type=password&client_id=bookcloud-app&client_secret=$CLIENT_SECRET&username=$1&password=password" \
    | jq -r '.access_token'
}

export CLIENT_SECRET
export TOKEN_READONLY; TOKEN_READONLY=$(_token readonlyuser)
export TOKEN_USER;     TOKEN_USER=$(_token testuser)
export TOKEN_ADMIN;    TOKEN_ADMIN=$(_token adminuser)
export BASE="$KONG_URL"

# ── Verify ────────────────────────────────────────────────────────────────────
_check() {
  local name="$1" token="$2"
  if [ -z "$token" ] || [ "$token" = "null" ]; then
    echo "✗ $name token is empty" >&2
    return 1
  fi
  echo "✓ $name: ${token:0:24}..."
}

_check "TOKEN_READONLY" "$TOKEN_READONLY"
_check "TOKEN_USER    " "$TOKEN_USER"
_check "TOKEN_ADMIN   " "$TOKEN_ADMIN"

cat <<EOF

All tokens and port-forwards ready.
  BASE=$BASE
  Keycloak PID=$PF_KEYCLOAK  |  Kong PID=$PF_KONG

To teardown port-forwards:
  kill $PF_KEYCLOAK $PF_KONG
EOF
