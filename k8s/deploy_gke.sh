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
INGRESS_HOST="${BOOKCLOUD_INGRESS_HOST:-api.bookcloud.local}"
NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
CREATE_CLUSTER="${GKE_CREATE_CLUSTER:-1}"

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

build_and_push() {
  local service="$1"
  local context="$2"
  local image="${IMAGE_PREFIX}/${service}:${IMAGE_TAG}"

  run docker build -t "$image" "$context"
  run docker push "$image"
}

replace_image() {
  local service="$1"
  local image="${IMAGE_PREFIX}/${service}:${IMAGE_TAG}"

  sed -i "s#image: bookcloud/${service}:latest#image: ${image}#g" \
    "$RENDER_DIR/${service}/deployment.yaml"
}

render_manifests() {
  cp -R "$SCRIPT_DIR"/. "$RENDER_DIR"/

  replace_image api-gateway
  replace_image book-catalog
  replace_image author-catalog
  replace_image rating-catalog
  replace_image compare-service
  replace_image genre-analysis-service

  find "$RENDER_DIR" -name deployment.yaml -print0 |
    xargs -0 sed -i 's/imagePullPolicy: IfNotPresent/imagePullPolicy: Always/g'

  sed -i "s/host: api.bookcloud.local/host: ${INGRESS_HOST}/g" \
    "$RENDER_DIR/api-gateway/ingress.yaml"
}

wait_for_rollouts() {
  local deployments
  local deployment

  deployments="$(kubectl -n "$NAMESPACE" get deployments -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}')"

  for deployment in $deployments; do
    run kubectl -n "$NAMESPACE" rollout status "deployment/$deployment" --timeout=600s
  done
}

print_ingress_info() {
  local address

  address="$(kubectl -n "$NAMESPACE" get ingress api-gateway -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || true)"

  cat <<EOF

BookCloud was deployed to GKE.

Project:      $PROJECT_ID
Region:       $REGION
Zone:         $ZONE
Cluster:      $CLUSTER_NAME
Repository:   $REPOSITORY
Image tag:    $IMAGE_TAG
Namespace:    $NAMESPACE
Ingress host: $INGRESS_HOST

Ingress external IP may take a few minutes to appear.
Check it with:
  kubectl -n $NAMESPACE get ingress api-gateway

EOF

  if [[ -n "$address" ]]; then
    cat <<EOF
Current ingress IP:
  $address

Test with:
  curl --resolve ${INGRESS_HOST}:80:${address} http://${INGRESS_HOST}/health

EOF
  fi
}

require_command gcloud
require_command docker
require_command kubectl

resolve_from_gcloud
require_project

REGISTRY_HOST="${REGION}-docker.pkg.dev"
IMAGE_PREFIX="${REGISTRY_HOST}/${PROJECT_ID}/${REPOSITORY}"

cd "$REPO_ROOT"

run gcloud config set project "$PROJECT_ID"
enable_services
ensure_artifact_registry
run gcloud auth configure-docker "$REGISTRY_HOST" --quiet
ensure_cluster

build_and_push api-gateway "$REPO_ROOT/src/api-gateway"
build_and_push book-catalog "$REPO_ROOT/src/services/book-catalog"
build_and_push author-catalog "$REPO_ROOT/src/services/author-catalog"
build_and_push rating-catalog "$REPO_ROOT/src/services/rating-catalog"
build_and_push compare-service "$REPO_ROOT/src/services/compare-service"
build_and_push genre-analysis-service "$REPO_ROOT/src/services/genre-analysis-service"

render_manifests

run kubectl apply -k "$RENDER_DIR"
wait_for_rollouts
run kubectl -n "$NAMESPACE" get pods,svc,ingress,hpa
print_ingress_info
