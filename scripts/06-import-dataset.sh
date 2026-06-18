#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/00-env.sh"

# ============================================================
# 6. Importar dataset
# ============================================================

resolve_public_url() {
  local namespace="$1"

  log "Resolver URL pública do Kong em ${namespace}"

  local external_ip=""

  for i in {1..60}; do
    external_ip="$(kubectl -n "$namespace" get svc kong \
      -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || true)"

    if [[ -n "$external_ip" ]]; then
      STAGING_BASE_URL="http://${external_ip}"
      export STAGING_BASE_URL

      echo "STAGING_BASE_URL=${STAGING_BASE_URL}"
      return 0
    fi

    echo "Kong ainda sem IP público. Tentativa ${i}/60..."
    kubectl -n "$namespace" get svc kong -o wide || true
    sleep 10
  done

  echo "Não foi possível resolver o IP público do Kong." >&2
  exit 1
}

download_dataset_from_gcs() {
  if [[ "$IMPORT_DATASET" != "true" ]]; then
    echo "IMPORT_DATASET=false. A saltar download do dataset."
    return 0
  fi

  log "Baixar dataset do Google Cloud Storage"

  echo "DATASET_GCS_URI=${DATASET_GCS_URI}"
  echo "DATASET_DIR=${DATASET_DIR}"

  if ! gcloud storage ls "$DATASET_GCS_URI" >/dev/null 2>&1; then
    echo "Dataset não encontrado em: ${DATASET_GCS_URI}" >&2
    echo ""
    echo "Para criar/popular o bucket:"
    echo "gcloud storage buckets create gs://${DATASET_BUCKET} --location=${ARTIFACT_REGISTRY_LOCATION} --default-storage-class=STANDARD --project=${GCP_PROJECT_ID}"
    echo "gcloud storage cp -r data/data_clean/normalized_out/* ${DATASET_GCS_URI}/"
    exit 1
  fi

  rm -rf "$DATASET_DIR"
  mkdir -p "$DATASET_DIR"

  gcloud storage cp -r "${DATASET_GCS_URI}/*" "$DATASET_DIR/"

  echo "Ficheiros baixados:"
  find "$DATASET_DIR" -maxdepth 2 -type f | sort
}

import_dataset() {
  if [[ "$IMPORT_DATASET" != "true" ]]; then
    echo "IMPORT_DATASET=false. A saltar importação do dataset."
    return 0
  fi

  log "Importar dataset"

  if [[ ! -d "$DATASET_DIR" ]]; then
    echo "DATASET_DIR não existe: ${DATASET_DIR}" >&2
    exit 1
  fi

  if [[ ! -f "k8s/import_data.sh" ]]; then
    echo "k8s/import_data.sh não encontrado." >&2
    exit 1
  fi

  chmod +x k8s/import_data.sh

  BOOKCLOUD_NAMESPACE="${K8S_NAMESPACE}" \
  BOOKCLOUD_CSV_DIR="${PWD}/${DATASET_DIR}" \
  BOOKCLOUD_DROP_BEFORE_IMPORT="${BOOKCLOUD_DROP_BEFORE_IMPORT:-0}" \
  ./k8s/import_data.sh
}

validate_local_environment

resolve_public_url "$K8S_NAMESPACE"
download_dataset_from_gcs
import_dataset

log "Dataset importado com sucesso"
