#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PROFILE="${BOOKCLOUD_MINIKUBE_PROFILE:-bookcloud}"
NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
MINIKUBE_NODES="${BOOKCLOUD_MINIKUBE_NODES:-1}"
ROLLOUT_TIMEOUT="${BOOKCLOUD_ROLLOUT_TIMEOUT:-300s}"

MINIKUBE_MEMORY="${BOOKCLOUD_MINIKUBE_MEMORY:-6144}"
MINIKUBE_CPUS="${BOOKCLOUD_MINIKUBE_CPUS:-4}"

KEYCLOAK_URL="http://localhost:8090"
KEYCLOAK_ADMIN="admin"
KEYCLOAK_ADMIN_PASSWORD="bookcloud-pass"
REALM="bookcloud"

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

build_image() {
  local image="$1"
  local context="$2"
  local dockerfile_path="${3:-$context/Dockerfile}"

  echo "--- Construindo imagem $image ---"
  echo "DEBUG: context=$context, dockerfile=$dockerfile_path"
  if ! docker build -t "$image" -f "$dockerfile_path" "$context"; then
    echo "Erro: falhou o build local da imagem '$image'." >&2
    exit 1
  fi

  echo "--- Carregando $image para o Minikube ---"
  if ! minikube image load "$image" -p "$PROFILE"; then
    echo "Erro: falhou o load da imagem para o Minikube." >&2
    exit 1
  fi
}

wait_for_rollouts() {
  local deployments
  local deployment

  deployments="$(kubectl -n "$NAMESPACE" get deployments -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}')"

  for deployment in $deployments; do
    run kubectl -n "$NAMESPACE" rollout status "deployment/$deployment" --timeout="$ROLLOUT_TIMEOUT"
  done
}

wait_for_keycloak() {
  echo "Waiting for Keycloak to be ready..."
  until curl -sf "$KEYCLOAK_URL/realms/master" >/dev/null 2>&1; do
    sleep 3
  done
  echo "Keycloak is ready."
}

setup_keycloak_users() {
  wait_for_keycloak

  # Get admin token
  TOKEN=$(curl -s -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
    -d "client_id=admin-cli&grant_type=password&username=$KEYCLOAK_ADMIN&password=$KEYCLOAK_ADMIN_PASSWORD" \
    | jq -r '.access_token')

  # Create users with roles
  # Format: "username:role"

  USERS_LIST=(
    "testuser:user"
    "adminuser:admin"
    "readonlyuser:readonly"
  ) 

  for entry in "${USERS_LIST[@]}"; do
    USERNAME="${entry%%:*}"
    ROLE="${entry##*:}"

    echo "Creating user: $USERNAME with role: $ROLE"

    # Create user (skip if exists)
    curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM/users" \
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
      }" || true

    # Get user ID
    USER_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/users?username=$USERNAME" \
      -H "Authorization: Bearer $TOKEN" | jq -r '.[0].id')

    # Get role ID
    ROLE_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM/roles/$ROLE" \
      -H "Authorization: Bearer $TOKEN" | jq -r '.id')

    # Assign role
    curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM/users/$USER_ID/role-mappings/realm" \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/json" \
      -d "[{\"id\": \"$ROLE_ID\", \"name\": \"$ROLE\"}]"

    echo "✓ $USERNAME ($ROLE)"
  done
}

require_command docker
require_command minikube
require_command kubectl
require_command curl

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

  helm repo add prometheus-community https://prometheus-community.github.io/helm-charts || true
  helm repo update

  helm upgrade --install monitoring prometheus-community/kube-prometheus-stack \
    --namespace monitoring \
    --values k8s/monitoring/values-local.yaml
}

cd "$REPO_ROOT"

if minikube -p "$PROFILE" status | grep -q "host: Running"; then
  echo "Minikube profile '$PROFILE' ja esta ativo."
else
  run minikube start -p "$PROFILE" \
  --nodes "$MINIKUBE_NODES" \
  --memory "$MINIKUBE_MEMORY" \
  --cpus "$MINIKUBE_CPUS"
fi

# Wait for API server to be fully ready before enabling addons
echo "Waiting for API server to be ready..."
until kubectl --context "$PROFILE" cluster-info >/dev/null 2>&1; do
  sleep 3
done
echo "API server is ready."

run minikube -p "$PROFILE" addons enable metrics-server

build_image "bookcloud/api-gateway:latest"           "$REPO_ROOT/src/api-gateway"
build_image "bookcloud/book-catalog:latest"          "$REPO_ROOT/src/services/book-catalog"
build_image "bookcloud/author-catalog:latest"        "$REPO_ROOT/src/services/author-catalog"
build_image "bookcloud/rating-catalog:latest"        "$REPO_ROOT/src/services/rating-catalog"
build_image "bookcloud/compare-service:latest"       "$REPO_ROOT/src/services/compare-service"
build_image "bookcloud/genre-analysis-service:latest" "$REPO_ROOT/src/services/genre-analysis-service"
build_image "bookcloud/book-search:latest"           "$REPO_ROOT"  "$REPO_ROOT/src/services/book-search/Dockerfile"
build_image "bookcloud/book-recommendation:latest"   "$REPO_ROOT/src/services/book-recommendation"
build_image "bookcloud/author-analytics-service:latest" "$REPO_ROOT/src/services/author-analytics-service"

install_monitoring_local

run kubectl apply -k "$SCRIPT_DIR"

# Regenerate realm ConfigMap from latest JSON
echo "Regenerating keycloak realm ConfigMap..."
kubectl create configmap keycloak-realm \
  --from-file=bookcloud-realm.json="$SCRIPT_DIR/keycloak/bookcloud-realm.json" \
  --namespace "$NAMESPACE" \
  --dry-run=client -o yaml \
  | kubectl apply -f -


wait_for_rollouts

# Port-forward Keycloak for local bootstrap
kubectl -n "$NAMESPACE" port-forward svc/keycloak 8090:80 &
PF_PID=$!

# Wait until the tunnel is actually accepting connections
echo "Waiting for port-forward to be ready..."
until curl -sf http://localhost:8090/realms/master >/dev/null 2>&1; do
  sleep 2
done

setup_keycloak_users

kill $PF_PID 2>/dev/null || true

run kubectl -n "$NAMESPACE" get pods,svc,hpa

cat <<EOF

BookCloud esta aplicado no namespace '$NAMESPACE'.

Prometheus:
  kubectl -n monitoring port-forward service/monitoring-kube-prometheus-prometheus 9090:9090
  http://localhost:9090

Grafana:
  kubectl -n monitoring port-forward service/monitoring-grafana 3000:80
  http://localhost:3000
  user: admin
  password: admin

Keycloak:
  kubectl -n bookcloud port-forward service/keycloak 8080:80
  http://localhost:8080
  user: admin
  password: bookcloud-pass

BookCloud via Kong:
  kubectl -n $NAMESPACE port-forward service/kong 9000:80
  http://localhost:9000

Acesso direto ao api-gateway, se precisares testar sem o Kong:
  kubectl -n $NAMESPACE port-forward service/api-gateway 8080:80
  curl http://localhost:8080/health

EOF
