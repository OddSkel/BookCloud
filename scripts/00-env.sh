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

# Compatibilidade com comandos/scripts antigos.
export NAMESPACE="${NAMESPACE:-${K8S_NAMESPACE}}"

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

export DATASET_BUCKET="${DATASET_BUCKET:-bookcloud_dataset}"
export DATASET_GCS_URI="${DATASET_GCS_URI:-gs://${DATASET_BUCKET}/normalized_out}"
export DATASET_DIR="${DATASET_DIR:-data/data_clean/normalized_out}"

export IMPORT_DATASET="${IMPORT_DATASET:-true}"
export SKIP_BUILD="${SKIP_BUILD:-false}"

# ============================================================
# Keycloak / Kong JWT
# ============================================================

export KEYCLOAK_SERVICE="${KEYCLOAK_SERVICE:-keycloak}"
export KEYCLOAK_SERVICE_PORT="${KEYCLOAK_SERVICE_PORT:-80}"
export LOCAL_KEYCLOAK_PORT="${LOCAL_KEYCLOAK_PORT:-18080}"
export KEYCLOAK_URL="${KEYCLOAK_URL:-http://127.0.0.1:${LOCAL_KEYCLOAK_PORT}}"

# O ficheiro bookcloud-realm.json exportado tem "realm": "bookcloud".
# Mantém este valor por defeito para alinhar Keycloak, api-gateway e Kong.
export KEYCLOAK_REALM="${KEYCLOAK_REALM:-${REALM:-bookcloud}}"
export REALM="${REALM:-${KEYCLOAK_REALM}}"
export KEYCLOAK_REALM_CANDIDATES="${KEYCLOAK_REALM_CANDIDATES:-bookcloud bookcloud-realm}"

export KEYCLOAK_ADMIN_USERNAME="${KEYCLOAK_ADMIN_USERNAME:-admin}"
export KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-bookcloud-pass}"

export KEYCLOAK_CLIENT_ID="${KEYCLOAK_CLIENT_ID:-bookcloud-app}"
export KEYCLOAK_CLIENT_SECRET="${KEYCLOAK_CLIENT_SECRET:-bookcloud-app-secret}"

export KEYCLOAK_TEST_PASSWORD="${KEYCLOAK_TEST_PASSWORD:-password}"
export KEYCLOAK_TEST_USERS="${KEYCLOAK_TEST_USERS:-testuser:user adminuser:admin readonlyuser:readonly}"
export KEYCLOAK_REALM_ROLES="${KEYCLOAK_REALM_ROLES:-user admin readonly}"

# Opcional. Se existir, o 07-keycloak.sh consegue importar o realm quando ele ainda não existe.
export KEYCLOAK_REALM_IMPORT_FILE="${KEYCLOAK_REALM_IMPORT_FILE:-bookcloud-realm.json}"

# Caminho relativo à raiz do repositório.
export KONG_CONFIGMAP_FILE="${KONG_CONFIGMAP_FILE:-k8s/kong/configmap.yaml}"

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
  echo "Namespace compat:         ${NAMESPACE}"
  echo "Namespace teste:          ${TEST_NAMESPACE}"
  echo "Namespace produção:       ${PROD_NAMESPACE}"

  echo "Kong IP name:             ${KONG_STATIC_IP_NAME}"
  echo "Kong IP region:           ${KONG_STATIC_IP_REGION}"

  echo "Keycloak service:         ${KEYCLOAK_SERVICE}:${KEYCLOAK_SERVICE_PORT}"
  echo "Keycloak local URL:       ${KEYCLOAK_URL}"
  echo "Keycloak realm:           ${REALM}"
  echo "Keycloak admin user:      ${KEYCLOAK_ADMIN_USERNAME}"
  echo "Keycloak client id:       ${KEYCLOAK_CLIENT_ID}"
  echo "Keycloak test users:      ${KEYCLOAK_TEST_USERS}"
  echo "Kong configmap file:      ${KONG_CONFIGMAP_FILE}"

  echo "Dataset GCS URI:          ${DATASET_GCS_URI}"
  echo "Dataset local dir:        ${DATASET_DIR}"

  echo "Import dataset:           ${IMPORT_DATASET}"
  echo "Skip build:               ${SKIP_BUILD}"
}
