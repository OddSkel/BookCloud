#!/usr/bin/env bash
# test_setup_cloud.sh — versão cloud (GKE / IP público).
# Usage: source ./k8s/test_setup_cloud.sh
#
# Não requer port-forwards — usa o IP público directamente.

CLOUD_IP="${BOOKCLOUD_CLOUD_IP:-146.148.2.198}"
export BASE="http://${CLOUD_IP}"
REALM="bookcloud"

echo "Using BASE=$BASE"

# ── Aguarda Kong responder ────────────────────────────────────────────────────
echo "Waiting for Kong at $BASE ..."
attempts=0
until curl -sf "$BASE/api/health" >/dev/null 2>&1; do
  sleep 2
  (( attempts++ )) || true
  if (( attempts > 20 )); then
    echo "✗ Kong did not respond at $BASE/api/health" >&2
    return 1
  fi
done
echo "✓ Kong ready."

# ── Obtém tokens via /api/auth/login ─────────────────────────────────────────
_token() {
  local username="$1" password="${2:-password}"
  curl -s -X POST "$BASE/api/auth/login" \\
    -H "Content-Type: application/x-www-form-urlencoded" \\
    -d "grant_type=password&client_id=bookcloud-app&username=${username}&password=${password}" \\
    | jq -r '.access_token'
}

export TOKEN_READONLY; TOKEN_READONLY=$(_token readonlyuser)
export TOKEN_USER;     TOKEN_USER=$(_token testuser)
export TOKEN_ADMIN;    TOKEN_ADMIN=$(_token adminuser)

# ── Valida tokens ─────────────────────────────────────────────────────────────
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

All tokens ready.
  BASE=$BASE

Usage:
  http GET  \\$BASE/api/books   "Authorization:Bearer \\$TOKEN_USER"
  http POST \\$BASE/api/auth/login username=testuser password=password
EOF
