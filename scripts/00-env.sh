#!/usr/bin/env bash

set -euo pipefail

# ============================================================
# BookCloud CD - Variáveis globais
# ============================================================

export ARTIFACT_REGISTRY_LOCATION="${ARTIFACT_REGISTRY_LOCATION:-europe-west1}"
export ARTIFACT_REGISTRY_REPOSITORY="${ARTIFACT_REGISTRY_REPOSITORY:-bookcloud}"

export GCP_PROJECT_ID="${GCP_PROJECT_ID:-cloud-computing-2526}"

export GKE_CLUSTER="${GKE_CLUSTER:-bookcloud-gke}"
export GKE_LOCATION="${GKE_LOCATION:-europe-west1}"
export GKE_MACHINE_TYPE="${GKE_MACHINE_TYPE:-e2-standard-2}"
export GKE_NUM_NODES="${GKE_NUM_NODES:-1}"
export GKE_DISK_SIZE_GB="${GKE_DISK_SIZE_GB:-30}"

export PROD_NAMESPACE="${PROD_NAMESPACE:-bookcloud}"
export TEST_NAMESPACE="${TEST_NAMESPACE:-bookcloud-test}"
export K8S_NAMESPACE="${K8S_NAMESPACE:-${TEST_NAMESPACE}}"

export PROD_KONG_STATIC_IP_NAME="${PROD_KONG_STATIC_IP_NAME:-bookcloud-kong-ip}"
export TEST_KONG_STATIC_IP_NAME="${TEST_KONG_STATIC_IP_NAME:-bookcloud-kong-test-ip}"
export KONG_STATIC_IP_REGION="${KONG_STATIC_IP_REGION:-europe-west1}"

if [[ -z "${KONG_STATIC_IP_NAME:-}" ]]; then
  if [[ "$K8S_NAMESPACE" == "$PROD_NAMESPACE" ]]; then
    export KONG_STATIC_IP_NAME="$PROD_KONG_STATIC_IP_NAME"
  else
    export KONG_STATIC_IP_NAME="$TEST_KONG_STATIC_IP_NAME"
  fi
fi

export ROLLOUT_TIMEOUT="${ROLLOUT_TIMEOUT:-600s}"

export IMAGE_PREFIX="${ARTIFACT_REGISTRY_LOCATION}-docker.pkg.dev/${GCP_PROJECT_ID}/${ARTIFACT_REGISTRY_REPOSITORY}"
export IMAGE_TAG="${IMAGE_TAG:-manual-$(git rev-parse --short HEAD 2>/dev/null || date +%s)}"

export DATASET_BUCKET="${DATASET_BUCKET:-bookcloud-dataset}"
export DATASET_GCS_URI="${DATASET_GCS_URI:-gs://${DATASET_BUCKET}/normalized_out}"
export DATASET_DIR="${DATASET_DIR:-data/data_clean/normalized_out}"

export IMPORT_DATASET="${IMPORT_DATASET:-true}"
export SKIP_BUILD="${SKIP_BUILD:-false}"

export DOCKER_BUILDKIT="1"

# ============================================================
# Helpers
# ============================================================

log() {
  echo ""
  echo "============================================================"
  echo "$1"
  echo "============================================================"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Comando obrigatório não encontrado: $1" >&2
    exit 1
  fi
}

require_file() {
  if [[ ! -f "$1" ]]; then
    echo "Ficheiro obrigatório não encontrado: $1" >&2
    exit 1
  fi
}

validate_local_environment() {
  log "Validar ambiente local"

  require_command gcloud
  require_command kubectl
  require_command jq
  require_command sed
  require_command curl
  require_command git
  require_command openssl
  require_command python3

  if [[ ! -d "k8s" ]]; then
    echo "Executa este script na raiz do repositório, onde existe a pasta k8s/." >&2
    exit 1
  fi

  if [[ ! -d "src" ]]; then
    echo "Executa este script na raiz do repositório, onde existe a pasta src/." >&2
    exit 1
  fi
}

print_cd_config() {
  log "Configuração atual do CD"

  echo "GCP project:              ${GCP_PROJECT_ID}"
  echo "Artifact Registry:        ${ARTIFACT_REGISTRY_REPOSITORY}"
  echo "Artifact location:        ${ARTIFACT_REGISTRY_LOCATION}"
  echo "Image prefix:             ${IMAGE_PREFIX}"
  echo "Image tag:                ${IMAGE_TAG}"

  echo "GKE cluster:              ${GKE_CLUSTER}"
  echo "GKE location:             ${GKE_LOCATION}"
  echo "GKE machine type:         ${GKE_MACHINE_TYPE}"
  echo "GKE num nodes:            ${GKE_NUM_NODES}"
  echo "GKE disk size:            ${GKE_DISK_SIZE_GB}GB"

  echo "Namespace atual:          ${K8S_NAMESPACE}"
  echo "Namespace teste:          ${TEST_NAMESPACE}"
  echo "Namespace produção:       ${PROD_NAMESPACE}"

  echo "Kong IP name:             ${KONG_STATIC_IP_NAME}"
  echo "Kong IP region:           ${KONG_STATIC_IP_REGION}"

  echo "Dataset GCS URI:          ${DATASET_GCS_URI}"
  echo "Dataset local dir:        ${DATASET_DIR}"

  echo "Import dataset:           ${IMPORT_DATASET}"
  echo "Skip build:               ${SKIP_BUILD}"
}