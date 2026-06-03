#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PROFILE="${BOOKCLOUD_MINIKUBE_PROFILE:-bookcloud}"
NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
MINIKUBE_NODES="${BOOKCLOUD_MINIKUBE_NODES:-1}"
ROLLOUT_TIMEOUT="${BOOKCLOUD_ROLLOUT_TIMEOUT:-600s}"
MINIKUBE_MEMORY="${BOOKCLOUD_MINIKUBE_MEMORY:-6144}"
MINIKUBE_CPUS="${BOOKCLOUD_MINIKUBE_CPUS:-4}"

KEYCLOAK_URL="http://localhost:8090"
KEYCLOAK_ADMIN="admin"
KEYCLOAK_ADMIN_PASSWORD="${BOOKCLOUD_KEYCLOAK_PASSWORD:-bookcloud-pass}"
REALM="bookcloud"

PF_KEYCLOAK_WATCHDOG_PID=""

# ── Cleanup trap ──────────────────────────────────────────────────────────────
cleanup() {
  [ -n "$PF_KEYCLOAK_WATCHDOG_PID" ] && kill "$PF_KEYCLOAK_WATCHDOG_PID" 2>/dev/null || true
  fuser -k 8090/tcp 2>/dev/null || true
  if [ -n "${MINIKUBE_DOCKER_ENV_SET:-}" ]; then
    eval "$(minikube -p "$PROFILE" docker-env --unset)" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# ── Helpers ───────────────────────────────────────────────────────────────────
require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Erro: comando '$1' nao encontrado." >&2
    exit 1
  fi
}

run() {
  echo "+ $*"
  "$@"
}

# Watchdog: keeps Keycloak port-forward alive across pod restarts.
start_keycloak_pf_watchdog() {
  fuser -k 8090/tcp 2>/dev/null || true
  (
    while true; do
      kubectl -n "$NAMESPACE" port-forward svc/keycloak 8090:80 2>/dev/null || true
      sleep 2
    done
  ) &
  PF_KEYCLOAK_WATCHDOG_PID=$!
  sleep 2
}

get_admin_token() {
  local token
  token=$(curl -s -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
    -d "client_id=admin-cli&grant_type=password&username=$KEYCLOAK_ADMIN&password=$KEYCLOAK_ADMIN_PASSWORD" \
    | jq -r '.access_token')
  if [ -z "$token" ] || [ "$token" = "null" ]; then
    echo "Erro: failed to obtain admin token from Keycloak" >&2
    return 1
  fi
  echo "$token"
}

create_user() {
  local USERNAME="$1"
  local ROLE="$2"
  local TOKEN
  TOKEN=$(get_admin_token)

  echo "Creating user: $USERNAME with role: $ROLE"

  local HTTP_STATUS
  HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$KEYCLOAK_URL/admin/realms/$REALM/users" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"username\": \"$USERNAME\",
      \"enabled\": true,
      \"credentials\": [{
        \"type\": \"password\",
        \"value\": \"password\",
        \"temporary\": false
      }]
    }")
  if [ "$HTTP_STATUS" != "201" ] && [ "$HTTP_STATUS" != "409" ]; then
    echo "Erro: failed to create user $USERNAME (HTTP $HTTP_STATUS)" >&2
    return 1
  fi

  local USER_ID
  USER_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/users?username=$USERNAME" \
    -H "Authorization: Bearer $TOKEN" | jq -r '.[0].id')
  if [ -z "$USER_ID" ] || [ "$USER_ID" = "null" ]; then
    echo "Erro: could not fetch UUID for user $USERNAME" >&2
    return 1
  fi

  local ROLE_ID
  ROLE_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/roles/$ROLE" \
    -H "Authorization: Bearer $TOKEN" | jq -r '.id')
  if [ -z "$ROLE_ID" ] || [ "$ROLE_ID" = "null" ]; then
    echo "Erro: could not fetch UUID for role $ROLE" >&2
    return 1
  fi

  HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$KEYCLOAK_URL/admin/realms/$REALM/users/$USER_ID/role-mappings/realm" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "[{\"id\": \"$ROLE_ID\", \"name\": \"$ROLE\"}]")
  if [ "$HTTP_STATUS" != "204" ]; then
    echo "Erro: failed to assign role $ROLE to $USERNAME (HTTP $HTTP_STATUS)" >&2
    return 1
  fi

  echo "✓ $USERNAME ($ROLE)"
}

# Waits for all deployments except Kong (Kong is patched and restarted separately).
wait_for_rollouts() {
  echo "Waiting for all deployments to roll out (excluding Kong)..."
  sleep 3

  local pids=()
  for deployment in $(kubectl -n "$NAMESPACE" get deployments \
      -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}'); do
    [ "$deployment" = "kong" ] && continue
    kubectl -n "$NAMESPACE" rollout status "deployment/$deployment" \
      --timeout="$ROLLOUT_TIMEOUT" &
    pids+=($!)
  done

  local failed=0
  for pid in "${pids[@]}"; do wait "$pid" || failed=1; done
  if [ "$failed" -eq 1 ]; then
    echo "Erro: one or more deployments failed to roll out." >&2
    exit 1
  fi
  echo "All deployments rolled out."
}

wait_for_keycloak() {
  echo "Waiting for Keycloak to be ready..."
  local attempts=0
  until curl -sf "$KEYCLOAK_URL/realms/master" >/dev/null 2>&1; do
    sleep 5
    (( attempts++ ))
    if (( attempts > 60 )); then
      echo "Erro: Keycloak did not become ready after ~5 minutes." >&2
      exit 1
    fi
  done
  echo "Keycloak is ready."
}

setup_keycloak_users() {
  wait_for_keycloak

  local TOKEN
  TOKEN=$(get_admin_token)

  local CLIENT_UUID
  CLIENT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/clients?clientId=bookcloud-app" \
    -H "Authorization: Bearer $TOKEN" | jq -r '.[0].id')
  if [ -z "$CLIENT_UUID" ] || [ "$CLIENT_UUID" = "null" ]; then
    echo "Erro: bookcloud-app client not found in realm '$REALM'." >&2
    exit 1
  fi

  local CLIENT_SECRET
  CLIENT_SECRET=$(curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM/clients/$CLIENT_UUID/client-secret" \
    -H "Authorization: Bearer $TOKEN" | jq -r '.value')
  echo "✓ bookcloud-app client secret: $CLIENT_SECRET"

  create_user "testuser"     "user"
  create_user "adminuser"    "admin"
  create_user "readonlyuser" "readonly"

  echo "✓ All users created."
  echo "$CLIENT_SECRET"
}

patch_kong_declarative_config() {
  echo "Fetching Keycloak RS256 signing key from JWKS..."
  local JWKS
  JWKS=$(curl -s "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/certs")

  local X5C
  X5C=$(echo "$JWKS" | jq -r '.keys[] | select(.alg=="RS256" and .use=="sig") | .x5c[0]')
  if [ -z "$X5C" ] || [ "$X5C" = "null" ]; then
    echo "Erro: failed to extract RS256 x5c certificate from JWKS." >&2
    exit 1
  fi

  local PUBKEY_PEM
  PUBKEY_PEM=$(echo "$X5C" \
    | base64 -d \
    | openssl x509 -inform DER -pubkey -noout 2>/dev/null)
  if [ -z "$PUBKEY_PEM" ]; then
    echo "Erro: failed to extract public key from x5c certificate." >&2
    exit 1
  fi

  local ISSUER="$KEYCLOAK_URL/realms/$REALM"
  local KONG_YML="$SCRIPT_DIR/kong/configmap.yaml"

  echo "Applying kong ConfigMap with live public key and issuer..."

  # Indent each PEM line by 14 spaces to match kong.yml block scalar indentation
  local INDENT="              "
  local INDENTED_PEM
  INDENTED_PEM=$(echo "$PUBKEY_PEM" | sed "s/^/${INDENT}/")

  # Replace placeholders in-memory and pipe directly to kubectl — file is never modified
  awk -v pem="$INDENTED_PEM" -v issuer="$ISSUER" '
    /KONG_JWT_PUBLIC_KEY_PLACEHOLDER/ { print pem; next }
    /KONG_JWT_ISSUER_PLACEHOLDER/     { sub(/KONG_JWT_ISSUER_PLACEHOLDER/, issuer) }
    { print }
  ' "$KONG_YML" | kubectl apply -f -

  echo "Restarting Kong to load new declarative config..."
  kubectl -n "$NAMESPACE" rollout restart deployment/kong
  kubectl -n "$NAMESPACE" rollout status deployment/kong --timeout="$ROLLOUT_TIMEOUT"
  echo "✓ Kong reloaded with live Keycloak public key."
}

install_helm_if_missing() {
  if command -v helm >/dev/null 2>&1; then
    echo "Helm already installed."
    return 0
  fi
  echo "Installing Helm..."
  curl -fsSL https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
}

install_monitoring_local() {
  install_helm_if_missing
  kubectl create namespace monitoring --dry-run=client -o yaml | kubectl apply -f -

  if kubectl -n monitoring get deployment monitoring-grafana &>/dev/null; then
    echo "Monitoring stack already installed, skipping."
    return 0
  fi

  if ! helm repo list 2>/dev/null | grep -q "prometheus-community"; then
    helm repo add prometheus-community https://prometheus-community.github.io/helm-charts \
      || { echo "Erro: failed to add prometheus-community Helm repo." >&2; exit 1; }
  fi
  helm repo update

  helm upgrade --install monitoring prometheus-community/kube-prometheus-stack \
    --namespace monitoring \
    --values "$SCRIPT_DIR/monitoring/values-local.yaml"
}

# ── Preflight checks ──────────────────────────────────────────────────────────
require_command docker
require_command minikube
require_command kubectl
require_command curl
require_command jq
require_command openssl

# ── Minikube ──────────────────────────────────────────────────────────────────
cd "$REPO_ROOT"

if minikube -p "$PROFILE" status | grep -q "host: Running"; then
  echo "Minikube profile '$PROFILE' ja esta ativo."
else
  run minikube start -p "$PROFILE" \
    --nodes "$MINIKUBE_NODES" \
    --memory "$MINIKUBE_MEMORY" \
    --cpus "$MINIKUBE_CPUS"
fi

echo "Waiting for API server to be ready..."
until kubectl --context "$PROFILE" cluster-info >/dev/null 2>&1; do
  sleep 3
done
echo "API server is ready."

minikube -p "$PROFILE" addons enable metrics-server || true

# ── Point Docker CLI at Minikube's daemon ─────────────────────────────────────
echo "Pointing Docker CLI at Minikube's internal daemon..."
eval "$(minikube -p "$PROFILE" docker-env)"
MINIKUBE_DOCKER_ENV_SET=1
export DOCKER_BUILDKIT=1

# ── Image builds (parallel) ───────────────────────────────────────────────────
echo "Building images directly into Minikube (parallel)..."
pids=()
for entry in \
  "bookcloud/api-gateway:latest|$REPO_ROOT/src/api-gateway|" \
  "bookcloud/book-catalog:latest|$REPO_ROOT/src/services/book-catalog|" \
  "bookcloud/author-catalog:latest|$REPO_ROOT/src/services/author-catalog|" \
  "bookcloud/rating-catalog:latest|$REPO_ROOT/src/services/rating-catalog|" \
  "bookcloud/compare-service:latest|$REPO_ROOT/src/services/compare-service|" \
  "bookcloud/genre-analysis-service:latest|$REPO_ROOT/src/services/genre-analysis-service|" \
  "bookcloud/author-analytics-service:latest|$REPO_ROOT/src/services/author-analytics-service|" \
  "bookcloud/book-recommendation:latest|$REPO_ROOT/src/services/book-recommendation|" \
  "bookcloud/book-search:latest|$REPO_ROOT|$REPO_ROOT/src/services/book-search/Dockerfile"
do
  image="${entry%%|*}"
  rest="${entry#*|}"
  context="${rest%%|*}"
  dockerfile="${rest##*|}"

  if [ -n "$dockerfile" ]; then
    docker build -f "$dockerfile" -t "$image" "$context" &
  else
    docker build -t "$image" "$context" &
  fi
  pids+=($!)
done

failed=0
for pid in "${pids[@]}"; do wait "$pid" || failed=1; done
[ "$failed" -eq 1 ] && { echo "Erro: one or more image builds failed." >&2; exit 1; }
echo "All images built into Minikube."

# ── Monitoring ────────────────────────────────────────────────────────────────
install_monitoring_local

# ── Kubernetes manifests ──────────────────────────────────────────────────────
# Kong is intentionally allowed to start with a placeholder/stale key here.
# It will be patched and restarted after Keycloak bootstrap below.
APPLY_OUT=$(kubectl apply -k "$SCRIPT_DIR" 2>&1)
echo "$APPLY_OUT"

echo "Regenerating keycloak realm ConfigMap..."
kubectl create configmap keycloak-realm \
  --from-file=bookcloud-realm.json="$SCRIPT_DIR/keycloak/bookcloud-realm.json" \
  --namespace "$NAMESPACE" \
  --dry-run=client -o yaml \
  | kubectl apply -f -

# ── Start Keycloak port-forward watchdog ──────────────────────────────────────
start_keycloak_pf_watchdog

# ── Wait for all non-Kong deployments ────────────────────────────────────────
if echo "$APPLY_OUT" | grep -qE '\s(created|configured)$'; then
  wait_for_rollouts
else
  echo "No deployments changed, skipping rollout wait."
fi

# ── Keycloak bootstrap ────────────────────────────────────────────────────────
_SECRET_FILE=$(mktemp)
setup_keycloak_users | tee "$_SECRET_FILE"
CLIENT_SECRET=$(tail -1 "$_SECRET_FILE")
rm -f "$_SECRET_FILE"

# ── Patch Kong configmap from template, restart Kong ─────────────────────────
patch_kong_declarative_config

# ── Stop Keycloak port-forward watchdog ──────────────────────────────────────
kill "$PF_KEYCLOAK_WATCHDOG_PID" 2>/dev/null || true
PF_KEYCLOAK_WATCHDOG_PID=""
fuser -k 8090/tcp 2>/dev/null || true

# ── Summary ───────────────────────────────────────────────────────────────────
run kubectl -n "$NAMESPACE" get pods,svc,hpa

cat <<'EOF'

✅ BookCloud is ready!
EOF
