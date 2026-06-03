#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PROFILE="${BOOKCLOUD_MINIKUBE_PROFILE:-bookcloud}"
NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
REMOVE_IMAGES="${BOOKCLOUD_REMOVE_IMAGES:-0}"

run() {
  echo "+ $*"
  "$@"
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# ── Kill any lingering port-forwards ─────────────────────────────────────────
kill_port_forwards() {
  echo "Killing any lingering port-forwards on 8090 and 8001..."
  fuser -k 8090/tcp 2>/dev/null || true
  fuser -k 8001/tcp 2>/dev/null || true
}

# ── Kubernetes resources ──────────────────────────────────────────────────────
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

# ── Monitoring stack ──────────────────────────────────────────────────────────
drop_monitoring() {
  if ! command_exists kubectl; then
    echo "kubectl not found; skipping monitoring resources."
    return 0
  fi

  if ! kubectl cluster-info >/dev/null 2>&1; then
    echo "No reachable Kubernetes cluster; skipping monitoring resources."
    return 0
  fi

  if command_exists helm; then
    helm uninstall monitoring --namespace monitoring 2>/dev/null || true
  fi

  kubectl delete namespace monitoring --ignore-not-found=true || true
}

# ── Minikube profile ──────────────────────────────────────────────────────────
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

# ── Docker images ─────────────────────────────────────────────────────────────
remove_bookcloud_images() {
  if [[ "$REMOVE_IMAGES" != "1" ]]; then
    echo "Keeping local Docker images. Set BOOKCLOUD_REMOVE_IMAGES=1 to remove bookcloud/* images."
    return 0
  fi

  if ! command_exists docker; then
    echo "docker not found; skipping image removal."
    return 0
  fi

  # Images live inside Minikube's daemon — point at it before removing
  # (minikube delete above already removed the VM, so images are gone anyway;
  #  this handles the case where REMOVE_IMAGES=1 is set without deleting minikube)
  local images
  images="$(docker images --format '{{.Repository}}:{{.Tag}}' | grep '^bookcloud/' || true)"
  if [[ -z "$images" ]]; then
    echo "No local bookcloud/* Docker images found."
    return 0
  fi

  echo "$images" | xargs -r docker rmi -f
  echo "✓ bookcloud/* images removed."
}

# ── Main ──────────────────────────────────────────────────────────────────────
cd "$REPO_ROOT"

kill_port_forwards
delete_kubernetes_resources
drop_monitoring
delete_minikube_profile
remove_bookcloud_images

cat <<'EOF'

✅ BookCloud dropped successfully.
EOF