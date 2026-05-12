#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PROJECT_ID="${GCP_PROJECT_ID:-${GOOGLE_CLOUD_PROJECT:-${CLOUDSDK_CORE_PROJECT:-${DEVSHELL_PROJECT_ID:-}}}}"
REGION="${GCP_REGION:-}"
ZONE="${GCP_ZONE:-}"
CLUSTER_NAME="${GKE_CLUSTER_NAME:-bookcloud-gke}"
REPOSITORY="${ARTIFACT_REGISTRY_REPOSITORY:-bookcloud}"
IMAGE_TAG="${BOOKCLOUD_IMAGE_TAG:-$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || date +%Y%m%d%H%M%S)}"
MACHINE_TYPE="${GKE_MACHINE_TYPE:-e2-standard-2}"
NUM_NODES="${GKE_NUM_NODES:-2}"
NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
CREATE_CLUSTER="${GKE_CREATE_CLUSTER:-1}"
INSTALL_MONITORING="${INSTALL_MONITORING:-1}"
MONITORING_NAMESPACE="${MONITORING_NAMESPACE:-monitoring}"
MONITORING_VALUES_FILE="${MONITORING_VALUES_FILE:-$SCRIPT_DIR/monitoring/values-gke.yaml}"

RENDER_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$RENDER_DIR"
}

trap cleanup EXIT

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Error: command '$1' not found." >&2
    exit 1
  fi
}

run() {
  echo "+ $*"
  "$@"
}

require_project() {
  if [[ -z "$PROJECT_ID" ]]; then
    cat >&2 <<EOF
Error: could not determine the GCP project automatically.

Set GCP_PROJECT_ID or run inside Cloud Shell / a configured gcloud session.

Example:
  GCP_PROJECT_ID=my-gcp-project ./k8s/deploy_gke.sh

EOF
    exit 1
  fi
}

resolve_from_gcloud() {
  local value

  if [[ -z "$PROJECT_ID" ]]; then
    value="$(gcloud config get-value project --quiet 2>/dev/null | tr -d '\r' || true)"
    [[ -n "$value" ]] && PROJECT_ID="$value"
  fi

  if [[ -z "$REGION" ]]; then
    value="$(gcloud config get-value compute/region --quiet 2>/dev/null | tr -d '\r' || true)"
    [[ -n "$value" ]] && REGION="$value"
  fi

  if [[ -z "$ZONE" ]]; then
    value="$(gcloud config get-value compute/zone --quiet 2>/dev/null | tr -d '\r' || true)"
    [[ -n "$value" ]] && ZONE="$value"
  fi

  [[ -z "$REGION" ]] && REGION="europe-west1"
  [[ -z "$ZONE" ]] && ZONE="europe-west1-b"
}

enable_services() {
  run gcloud services enable \
    container.googleapis.com \
    artifactregistry.googleapis.com \
    compute.googleapis.com \
    cloudbuild.googleapis.com \
    --project "$PROJECT_ID"
}

ensure_artifact_registry() {
  if gcloud artifacts repositories describe "$REPOSITORY" \
    --location "$REGION" \
    --project "$PROJECT_ID" >/dev/null 2>&1; then
    echo "Artifact Registry repository '$REPOSITORY' already exists in '$REGION'."
    return 0
  fi

  run gcloud artifacts repositories create "$REPOSITORY" \
    --repository-format=docker \
    --location="$REGION" \
    --description="BookCloud container images" \
    --project "$PROJECT_ID"
}

ensure_cluster() {
  if gcloud container clusters describe "$CLUSTER_NAME" \
    --zone "$ZONE" \
    --project "$PROJECT_ID" >/dev/null 2>&1; then
    echo "GKE cluster '$CLUSTER_NAME' already exists in '$ZONE'."
  elif [[ "$CREATE_CLUSTER" == "1" ]]; then
    run gcloud container clusters create "$CLUSTER_NAME" \
      --zone "$ZONE" \
      --num-nodes "$NUM_NODES" \
      --machine-type "$MACHINE_TYPE" \
      --enable-ip-alias \
      --project "$PROJECT_ID"
  else
    echo "Error: cluster '$CLUSTER_NAME' does not exist and GKE_CREATE_CLUSTER=0." >&2
    exit 1
  fi

  run gcloud container clusters get-credentials "$CLUSTER_NAME" \
    --zone "$ZONE" \
    --project "$PROJECT_ID"
}

install_helm_if_missing() {
  if command -v helm >/dev/null 2>&1; then
    echo "Helm already installed."
    return 0
  fi

  echo "Installing Helm..."
  curl -fsSL https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
}

install_monitoring() {
  if [[ "${INSTALL_MONITORING}" != "1" ]]; then
    echo "Skipping monitoring installation because INSTALL_MONITORING=${INSTALL_MONITORING}."
    return 0
  fi

  install_helm_if_missing

  kubectl create namespace "$MONITORING_NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

  run helm repo add prometheus-community https://prometheus-community.github.io/helm-charts || true
  run helm repo update

  run helm upgrade --install monitoring prometheus-community/kube-prometheus-stack \
    --namespace "$MONITORING_NAMESPACE" \
    --values "$MONITORING_VALUES_FILE"
}

build_and_push() {
  local service="$1"
  local context="$2"
  local image="${IMAGE_PREFIX}/${service}:${IMAGE_TAG}"

  run gcloud builds submit "$context" \
    --tag "$image" \
    --project "$PROJECT_ID"
}

replace_image() {
  local service="$1"
  local image="${IMAGE_PREFIX}/${service}:${IMAGE_TAG}"
  local deployment_file="$RENDER_DIR/${service}/deployment.yaml"

  if [[ ! -f "$deployment_file" ]]; then
    echo "Warning: deployment file not found for service '$service': $deployment_file" >&2
    return 0
  fi

  sed -i "s#image: bookcloud/${service}:latest#image: ${image}#g" "$deployment_file"
}

force_api_gateway_cluster_ip() {
  local service_file="$RENDER_DIR/api-gateway/service.yaml"

  if [[ ! -f "$service_file" ]]; then
    echo "Warning: api-gateway service file not found: $service_file" >&2
    return 0
  fi

  sed -i 's/type: LoadBalancer/type: ClusterIP/g' "$service_file"
  sed -i 's/type: NodePort/type: ClusterIP/g' "$service_file"
}

force_kong_load_balancer() {
  local service_file="$RENDER_DIR/kong/service.yaml"

  if [[ ! -f "$service_file" ]]; then
    echo "Warning: kong service file not found: $service_file" >&2
    return 0
  fi

  sed -i 's/type: ClusterIP/type: LoadBalancer/g' "$service_file"
  sed -i 's/type: NodePort/type: LoadBalancer/g' "$service_file"
}

render_manifests() {
  cp -R "$SCRIPT_DIR"/. "$RENDER_DIR"/

  replace_image api-gateway
  replace_image book-catalog
  replace_image author-catalog
  replace_image rating-catalog
  replace_image compare-service
  replace_image genre-analysis-service

  force_api_gateway_cluster_ip
  force_kong_load_balancer

  find "$RENDER_DIR" -name deployment.yaml -print0 |
    xargs -0 sed -i 's/imagePullPolicy: IfNotPresent/imagePullPolicy: Always/g'
}

wait_for_rollouts() {
  local deployments
  local deployment

  deployments="$(kubectl -n "$NAMESPACE" get deployments -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}')"

  for deployment in $deployments; do
    run kubectl -n "$NAMESPACE" rollout status "deployment/$deployment" --timeout=600s
  done
}

print_kong_access() {
  echo
  echo "Waiting for Kong external IP..."

  run kubectl -n "$NAMESPACE" get svc kong

  local external_ip=""

  for _ in {1..60}; do
    external_ip="$(kubectl -n "$NAMESPACE" get svc kong -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || true)"

    if [[ -z "$external_ip" ]]; then
      external_ip="$(kubectl -n "$NAMESPACE" get svc kong -o jsonpath='{.status.loadBalancer.ingress[0].hostname}' 2>/dev/null || true)"
    fi

    if [[ -n "$external_ip" ]]; then
      break
    fi

    sleep 5
  done

  echo

  if [[ -n "$external_ip" ]]; then
    echo "Kong is exposed externally at:"
    echo "  http://${external_ip}"
    echo
    echo "Example:"
    echo "  curl http://${external_ip}/api/health"
    echo "  curl http://${external_ip}/api/books"
  else
    echo "Kong LoadBalancer external IP is still pending."
    echo
    echo "Check with:"
    echo "  kubectl -n ${NAMESPACE} get svc kong -w"
    echo
    echo "Temporary local access:"
    echo "  kubectl -n ${NAMESPACE} port-forward service/kong 9000:80"
    echo "  curl http://localhost:9000/api/health"
    echo "  curl http://localhost:9000/api/books"
  fi
}

print_monitoring_access() {
  echo
  echo "Monitoring services:"
  kubectl -n "$MONITORING_NAMESPACE" get svc

  echo
  echo "Grafana external access:"
  kubectl -n "$MONITORING_NAMESPACE" get svc monitoring-grafana || true

  echo
  echo "Prometheus local access:"
  echo "  kubectl -n $MONITORING_NAMESPACE port-forward service/monitoring-kube-prometheus-prometheus 9090:9090"
  echo "  http://localhost:9090"
}

require_command gcloud
require_command kubectl
require_command curl

resolve_from_gcloud
require_project

REGISTRY_HOST="${REGION}-docker.pkg.dev"
IMAGE_PREFIX="${REGISTRY_HOST}/${PROJECT_ID}/${REPOSITORY}"

cd "$REPO_ROOT"

run gcloud config set project "$PROJECT_ID"
enable_services
ensure_artifact_registry
ensure_cluster
install_monitoring

build_and_push api-gateway "$REPO_ROOT/src/api-gateway"
build_and_push book-catalog "$REPO_ROOT/src/services/book-catalog"
build_and_push author-catalog "$REPO_ROOT/src/services/author-catalog"
build_and_push rating-catalog "$REPO_ROOT/src/services/rating-catalog"
build_and_push compare-service "$REPO_ROOT/src/services/compare-service"
build_and_push genre-analysis-service "$REPO_ROOT/src/services/genre-analysis-service"

render_manifests

run kubectl apply -k "$RENDER_DIR"
wait_for_rollouts
run kubectl -n "$NAMESPACE" get pods,svc,hpa
print_kong_access
print_monitoring_access
