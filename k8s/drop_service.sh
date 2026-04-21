#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PROFILE="${BOOKCLOUD_MINIKUBE_PROFILE:-bookcloud}"
NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
REMOVE_IMAGES="${BOOKCLOUD_REMOVE_IMAGES:-0}"

COMPOSE_FILES=(
  "$REPO_ROOT/data/data_clean/docker-compose.yaml"
  "$REPO_ROOT/src/api-gateway/docker-compose.yml"
  "$REPO_ROOT/src/services/author-analytics-service/docker-compose.yml"
  "$REPO_ROOT/src/services/author-catalog/docker-compose.yml"
  "$REPO_ROOT/src/services/book-catalog/docker-compose.yml"
  "$REPO_ROOT/src/services/book-recommendation/docker-compose.yml"
  "$REPO_ROOT/src/services/book-search/docker-compose.yml"
  "$REPO_ROOT/src/services/compare-service/docker-compose.yml"
  "$REPO_ROOT/src/services/genre-analysis-service/docker-compose.yml"
  "$REPO_ROOT/src/services/rating-catalog/docker-compose.yml"
)

run() {
  echo "+ $*"
  "$@"
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

delete_kubernetes_resources() {
  if ! command_exists kubectl; then
    echo "kubectl not found; skipping Kubernetes resources."
    return 0
  fi

  if kubectl cluster-info >/dev/null 2>&1; then
    run kubectl delete -k "$SCRIPT_DIR" --ignore-not-found=true || true
    run kubectl delete namespace "$NAMESPACE" --ignore-not-found=true || true
  else
    echo "No reachable Kubernetes cluster; skipping kubectl delete."
  fi
}

delete_minikube_profile() {
  if ! command_exists minikube; then
    echo "minikube not found; skipping Minikube profile."
    return 0
  fi

  if minikube -p "$PROFILE" status >/dev/null 2>&1; then
    run minikube delete -p "$PROFILE"
  else
    echo "Minikube profile '$PROFILE' is not running or does not exist."
  fi
}

docker_compose_down() {
  local compose_file

  if ! command_exists docker; then
    echo "docker not found; skipping Docker Compose."
    return 0
  fi

  for compose_file in "${COMPOSE_FILES[@]}"; do
    if [[ -f "$compose_file" ]]; then
      run docker compose -f "$compose_file" down --remove-orphans || true
    fi
  done
}

remove_bookcloud_images() {
  local images

  if [[ "$REMOVE_IMAGES" != "1" ]]; then
    echo "Keeping local Docker images. Set BOOKCLOUD_REMOVE_IMAGES=1 to remove bookcloud/* images."
    return 0
  fi

  if ! command_exists docker; then
    echo "docker not found; skipping image removal."
    return 0
  fi

  images="$(docker images --format '{{.Repository}}:{{.Tag}}' | grep '^bookcloud/' || true)"
  if [[ -z "$images" ]]; then
    echo "No local bookcloud/* Docker images found."
    return 0
  fi

  echo "$images" | xargs -r docker rmi -f
}

cd "$REPO_ROOT"

delete_kubernetes_resources
delete_minikube_profile
docker_compose_down
remove_bookcloud_images

cat <<EOF

BookCloud local services have been dropped.

Removed:
  - Kubernetes resources from namespace '$NAMESPACE'
  - Minikube profile '$PROFILE', if present
  - Docker Compose containers from this repository

EOF
