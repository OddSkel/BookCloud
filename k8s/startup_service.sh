#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PROFILE="${BOOKCLOUD_MINIKUBE_PROFILE:-bookcloud}"
NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
MINIKUBE_NODES="${BOOKCLOUD_MINIKUBE_NODES:-1}"
ROLLOUT_TIMEOUT="${BOOKCLOUD_ROLLOUT_TIMEOUT:-300s}"

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
  local log_file

  log_file="$(mktemp)"
  echo "+ minikube -p $PROFILE image build -t $image $context"
  if ! minikube -p "$PROFILE" image build -t "$image" "$context" 2>&1 | tee "$log_file"; then
    echo "Erro: falhou o build da imagem '$image'." >&2
    rm -f "$log_file"
    exit 1
  fi

  if grep -qE "ERROR: failed to build|error: could not compile" "$log_file"; then
    echo "Erro: o build da imagem '$image' terminou com erros." >&2
    rm -f "$log_file"
    exit 1
  fi

  rm -f "$log_file"
}

wait_for_rollouts() {
  local deployments
  local deployment

  deployments="$(kubectl -n "$NAMESPACE" get deployments -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}')"

  for deployment in $deployments; do
    run kubectl -n "$NAMESPACE" rollout status "deployment/$deployment" --timeout="$ROLLOUT_TIMEOUT"
  done
}

require_command docker
require_command minikube
require_command kubectl

cd "$REPO_ROOT"

if minikube -p "$PROFILE" status >/dev/null 2>&1; then
  echo "Minikube profile '$PROFILE' ja esta ativo."
else
  run minikube start -p "$PROFILE" --nodes "$MINIKUBE_NODES"
fi

run minikube -p "$PROFILE" addons enable metrics-server

build_image "bookcloud/api-gateway:latest" "$REPO_ROOT/src/api-gateway"
build_image "bookcloud/book-catalog:latest" "$REPO_ROOT/src/services/book-catalog"
build_image "bookcloud/author-catalog:latest" "$REPO_ROOT/src/services/author-catalog"
build_image "bookcloud/rating-catalog:latest" "$REPO_ROOT/src/services/rating-catalog"
build_image "bookcloud/compare-service:latest" "$REPO_ROOT/src/services/compare-service"
build_image "bookcloud/genre-analysis-service:latest" "$REPO_ROOT/src/services/genre-analysis-service"

run kubectl apply -k "$SCRIPT_DIR"

wait_for_rollouts

run kubectl -n "$NAMESPACE" get pods,svc,hpa

cat <<EOF

BookCloud esta aplicado no namespace '$NAMESPACE'.

Kong Gateway:
  kubectl -n $NAMESPACE port-forward svc/kong 8000:80
  curl http://localhost:8000/health

Acesso direto ao api-gateway, se precisares testar sem o Kong:
  kubectl -n $NAMESPACE port-forward svc/api-gateway 8080:80
  curl http://localhost:8080/health

EOF