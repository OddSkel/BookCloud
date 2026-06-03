#!/usr/bin/env bash
# test_setup_cloud.sh — versão cloud (GKE / IP público)
# Usage: source ./k8s/test_setup_cloud.sh

CLOUD_IP="${BOOKCLOUD_CLOUD_IP:-146.148.2.198}"
KEYCLOAK_URL="http://${CLOUD_IP}/keycloak"   # ajusta o path se necessário
KONG_URL="http://${CLOUD_IP}"
REALM="bookcloud"

# ── Verificar conectividade ────────────────────────────────────────────────────
echo "Waiting for Keycloak at $KEYCLOAK_URL ..."
attempts=0
until curl -sf "$KEYCLOAK_URL/realms/master" >/dev/null 2>&1; do
  sleep 2
  (( attempts++ )) || true
  if (( attempts > 20 )); then
    echo "✗ Keycloak did not respond at $KEYCLOAK_URL" >&2
    return 1
  fi
done
echo "✓ Keycloak ready."

# ── Resolve client secret ──────────────────────────────────────────────────────
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
  echo "✗ Could not resolve client secret." >&2
  return 1
fi
echo "✓ Client secret resolved."

# ── Fetch tokens ───────────────────────────────────────────────────────────────
_token() {
  curl -s -X POST "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/token" \
    -d "grant_type=password&client_id=bookcloud-app&client_secret=$CLIENT_SECRET&username=$1&password=password" \
    | jq -r '.access_token'
}

export TOKEN_READONLY; TOKEN_READONLY=$(_token readonlyuser)
export TOKEN_USER;     TOKEN_USER=$(_token testuser)
export TOKEN_ADMIN;    TOKEN_ADMIN=$(_token adminuser)
export BASE="$KONG_URL"
export CLIENT_SECRET

echo "✓ TOKEN_READONLY: ${TOKEN_READONLY:0:24}..."
echo "✓ TOKEN_USER    : ${TOKEN_USER:0:24}..."
echo "✓ TOKEN_ADMIN   : ${TOKEN_ADMIN:0:24}..."
echo ""
echo "All tokens ready."
echo "  BASE=$BASE"
echo ""
