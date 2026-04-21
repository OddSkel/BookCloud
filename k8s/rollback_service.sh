#!/usr/bin/env bash
set -euo pipefail

NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
ROLLOUT_TIMEOUT="${BOOKCLOUD_ROLLOUT_TIMEOUT:-300s}"

DEPLOYMENTS=(
  api-gateway
  book-catalog
  author-catalog
  rating-catalog
  compare-service
  genre-analysis-service
)

run() {
  echo "+ $*"
  "$@"
}

if ! command -v kubectl >/dev/null 2>&1; then
  echo "Error: command 'kubectl' not found." >&2
  exit 1
fi

for deployment in "${DEPLOYMENTS[@]}"; do
  run kubectl -n "$NAMESPACE" rollout undo "deployment/$deployment"
done

for deployment in "${DEPLOYMENTS[@]}"; do
  run kubectl -n "$NAMESPACE" rollout status "deployment/$deployment" --timeout="$ROLLOUT_TIMEOUT"
done

run kubectl -n "$NAMESPACE" get deployments,hpa,pods

cat <<EOF

Rollback finished for BookCloud microservices in namespace '$NAMESPACE'.

EOF
