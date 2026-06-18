#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/00-env.sh"

# ============================================================
# 2. Garantir cluster Kubernetes e obter acesso ao cluster
# ============================================================

ensure_gke_cluster() {
  log "Garantir cluster GKE"

  local cluster_status=""

  if gcloud container clusters describe "$GKE_CLUSTER" \
    --location="$GKE_LOCATION" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    cluster_status="$(gcloud container clusters describe "$GKE_CLUSTER" \
      --location="$GKE_LOCATION" \
      --project="$GCP_PROJECT_ID" \
      --format="value(status)")"

    echo "Cluster existe com status: ${cluster_status}"

    if [[ "$cluster_status" != "RUNNING" ]]; then
      echo "Cluster existe, mas não está RUNNING." >&2
      echo "Apaga manualmente ou usa drop-all.sh antes de recriar." >&2
      exit 1
    fi
  else
    echo "Cluster não existe. A criar cluster..."

    gcloud container clusters create "$GKE_CLUSTER" \
      --location="$GKE_LOCATION" \
      --num-nodes="$GKE_NUM_NODES" \
      --machine-type="$GKE_MACHINE_TYPE" \
      --disk-type="pd-standard" \
      --disk-size="$GKE_DISK_SIZE_GB" \
      --enable-ip-alias \
      --project="$GCP_PROJECT_ID"
  fi

  echo "A aguardar cluster RUNNING..."

  for i in {1..60}; do
    cluster_status="$(gcloud container clusters describe "$GKE_CLUSTER" \
      --location="$GKE_LOCATION" \
      --project="$GCP_PROJECT_ID" \
      --format="value(status)")"

    echo "Cluster status: ${cluster_status}"

    if [[ "$cluster_status" == "RUNNING" ]]; then
      echo "Cluster GKE está RUNNING."
      return 0
    fi

    sleep 10
  done

  echo "Cluster não ficou RUNNING a tempo." >&2
  exit 1
}

get_gke_credentials() {
  log "Obter credenciais do GKE"

  gcloud container clusters get-credentials "$GKE_CLUSTER" \
    --location="$GKE_LOCATION" \
    --project="$GCP_PROJECT_ID"

  echo "A testar acesso à API Kubernetes..."

  for i in {1..60}; do
    if kubectl version --request-timeout=10s >/dev/null 2>&1; then
      echo "Kubernetes API acessível."
      return 0
    fi

    echo "Kubernetes API ainda não respondeu. Tentativa ${i}/60..."
    sleep 10
  done

  echo "Kubernetes API não ficou acessível a tempo." >&2
  exit 1
}

validate_local_environment

ensure_gke_cluster
get_gke_credentials

log "Cluster Kubernetes e acesso configurados com sucesso"