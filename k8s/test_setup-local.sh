#!/usr/bin/env bash
# test_setup-local.sh — prepara port-forwards e tokens para testes locais.
# Usage: source ./k8s/test_setup-local.sh

NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
BASE_LOCAL="http://localhost:9000"

# ── Limpa port-forwards anteriores ───────────────────────────────────────────
echo "Cleaning up stale port-forwards..."
fuser -k 9000/tcp 2>/dev/null || true
sleep 1

# ── Inicia port-forward só para o Kong ───────────────────────────────────────
echo "Starting port-forward..."
kubectl -n "$NAMESPACE" port-forward svc/kong 9000:80 >/dev/null 2>&1 &
PF_KONG=$!
sleep 3

export BASE="$BASE_LOCAL"

# ── Obtém tokens via /api/auth/login (Kong → Keycloak) ───────────────────────
echo "Fetching tokens via /api/auth/login..."

_token() {
  local username="$1" password="${2:-password}"
  curl -s -X POST "$BASE/api/auth/login" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=password&client_id=bookcloud-app&username=${username}&password=${password}" \
    | jq -r '.access_token'
}

export TOKEN_READONLY; TOKEN_READONLY=$(_token readonlyuser)
export TOKEN_USER;     TOKEN_USER=$(_token testuser)
export TOKEN_ADMIN;    TOKEN_ADMIN=$(_token adminuser)

# ── Valida tokens ─────────────────────────────────────────────────────────────
_check() {
  local name="$1" token="$2"
  if [ -z "$token" ] || [ "$token" = "null" ]; then
    echo "✗ $name token is empty — Kong /api/auth/login failed?" >&2
    return 1
  fi
  echo "✓ $name: ${token:0:24}..."
}

_check "TOKEN_READONLY" "$TOKEN_READONLY"
_check "TOKEN_USER    " "$TOKEN_USER"
_check "TOKEN_ADMIN   " "$TOKEN_ADMIN"

echo ""
echo "All tokens and port-forwards ready."
echo "  BASE=$BASE"
echo "  Kong PID=$PF_KONG"
echo ""
echo "Usage:"
echo "  http GET  $BASE/api/books   \"Authorization:Bearer \$TOKEN_USER\""
echo "  http POST $BASE/api/auth/login username=testuser password=password"
echo ""
echo "To teardown:"
echo "  kill $PF_KONG"
