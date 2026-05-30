#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${PROJECT_ID:-cloud-computing-2526}"
GKE_CLUSTER="${GKE_CLUSTER:-bookcloud-gke}"
GKE_LOCATION="${GKE_LOCATION:-europe-west1}"
K8S_NAMESPACE="${K8S_NAMESPACE:-bookcloud}"
MONITORING_NAMESPACE="${MONITORING_NAMESPACE:-monitoring}"
ARTIFACT_REGISTRY_LOCATION="${ARTIFACT_REGISTRY_LOCATION:-europe-west1}"
ARTIFACT_REGISTRY_REPOSITORY="${ARTIFACT_REGISTRY_REPOSITORY:-bookcloud}"
KONG_STATIC_IP_NAME="${KONG_STATIC_IP_NAME:-bookcloud-kong-ip}"
KONG_STATIC_IP_REGION="${KONG_STATIC_IP_REGION:-$GKE_LOCATION}"
DELETE_STATIC_IP="${DELETE_STATIC_IP:-false}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command not found: $1" >&2
    exit 1
  fi
}

cluster_exists() {
  gcloud container clusters describe "$GKE_CLUSTER" \
    --location "$GKE_LOCATION" \
    --project "$PROJECT_ID" >/dev/null 2>&1
}

delete_namespace_if_present() {
  local namespace="$1"

  if kubectl get namespace "$namespace" >/dev/null 2>&1; then
    echo "Deleting namespace: $namespace"
    kubectl delete namespace "$namespace" --ignore-not-found=true
  else
    echo "Namespace not found, skipping: $namespace"
  fi
}

delete_orphan_disks() {
  echo "Looking for orphaned BookCloud/GKE disks..."

  gcloud compute disks list \
    --project "$PROJECT_ID" \
    --filter="(name~'bookcloud' OR name~'gke-${GKE_CLUSTER}' OR name~'gke-.*pvc') AND users=null" \
    --format="value(name,zone)" |
  while read -r disk zone_url; do
    [[ -n "${disk:-}" && -n "${zone_url:-}" ]] || continue

    zone="${zone_url##*/}"
    echo "Deleting orphan disk: ${disk} (${zone})"
    gcloud compute disks delete "$disk" \
      --zone "$zone" \
      --project "$PROJECT_ID" \
      --quiet || true
  done
}

delete_firewall_rules() {
  echo "Looking for BookCloud/GKE firewall rules..."

  gcloud compute firewall-rules list \
    --project "$PROJECT_ID" \
    --filter="name~'bookcloud' OR name~'gke-${GKE_CLUSTER}' OR name~'k8s'" \
    --format="value(name)" |
  while read -r rule; do
    [[ -n "${rule:-}" ]] || continue

    echo "Deleting firewall rule: ${rule}"
    gcloud compute firewall-rules delete "$rule" \
      --project "$PROJECT_ID" \
      --quiet || true
  done
}

delete_kubectl_context() {
  local context="gke_${PROJECT_ID}_${GKE_LOCATION}_${GKE_CLUSTER}"

  echo "Cleaning local kubectl context: ${context}"
  kubectl config delete-context "$context" >/dev/null 2>&1 || true
  kubectl config delete-cluster "$context" >/dev/null 2>&1 || true
}

require_command gcloud
require_command kubectl

cat <<EOF
This will delete the BookCloud cloud environment:
  project:                 ${PROJECT_ID}
  cluster:                 ${GKE_CLUSTER}
  cluster location:        ${GKE_LOCATION}
  app namespace:           ${K8S_NAMESPACE}
  monitoring namespace:    ${MONITORING_NAMESPACE}
  artifact repository:     ${ARTIFACT_REGISTRY_LOCATION}/${ARTIFACT_REGISTRY_REPOSITORY}
  kong static IP:          ${KONG_STATIC_IP_REGION}/${KONG_STATIC_IP_NAME}
  delete static IP:        ${DELETE_STATIC_IP}

Type DELETE to continue.
EOF

read -r confirmation

if [[ "$confirmation" != "DELETE" ]]; then
  echo "Aborted."
  exit 1
fi

gcloud config set project "$PROJECT_ID"

if cluster_exists; then
  echo "Getting credentials for cluster: ${GKE_CLUSTER}"
  gcloud container clusters get-credentials "$GKE_CLUSTER" \
    --location "$GKE_LOCATION" \
    --project "$PROJECT_ID"

  delete_namespace_if_present "$K8S_NAMESPACE"
  delete_namespace_if_present "$MONITORING_NAMESPACE"

  echo "Waiting for LoadBalancer cleanup to start..."
  sleep 45

  echo "Deleting GKE cluster: ${GKE_CLUSTER}"
  gcloud container clusters delete "$GKE_CLUSTER" \
    --location "$GKE_LOCATION" \
    --project "$PROJECT_ID" \
    --quiet
else
  echo "GKE cluster not found, skipping: ${GKE_CLUSTER}"
fi

if gcloud artifacts repositories describe "$ARTIFACT_REGISTRY_REPOSITORY" \
  --location "$ARTIFACT_REGISTRY_LOCATION" \
  --project "$PROJECT_ID" >/dev/null 2>&1; then
  echo "Deleting Artifact Registry repository: ${ARTIFACT_REGISTRY_REPOSITORY}"
  gcloud artifacts repositories delete "$ARTIFACT_REGISTRY_REPOSITORY" \
    --location "$ARTIFACT_REGISTRY_LOCATION" \
    --project "$PROJECT_ID" \
    --quiet
else
  echo "Artifact Registry repository not found, skipping: ${ARTIFACT_REGISTRY_REPOSITORY}"
fi

delete_orphan_disks
delete_firewall_rules

if [[ "$DELETE_STATIC_IP" == "true" ]]; then
  if gcloud compute addresses describe "$KONG_STATIC_IP_NAME" \
    --region "$KONG_STATIC_IP_REGION" \
    --project "$PROJECT_ID" >/dev/null 2>&1; then
    echo "Deleting Kong static IP: ${KONG_STATIC_IP_NAME}"
    gcloud compute addresses delete "$KONG_STATIC_IP_NAME" \
      --region "$KONG_STATIC_IP_REGION" \
      --project "$PROJECT_ID" \
      --quiet
  else
    echo "Kong static IP not found, skipping: ${KONG_STATIC_IP_NAME}"
  fi
else
  echo "Preserving Kong static IP: ${KONG_STATIC_IP_NAME}"
fi

delete_kubectl_context

echo "BookCloud cloud teardown completed."
