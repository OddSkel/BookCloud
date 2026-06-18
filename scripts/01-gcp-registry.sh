#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/00-env.sh"

# ============================================================
# 1. Configurar GCP, ativar APIs, permissões e registry Docker
# ============================================================

configure_gcp() {
  log "Configurar projeto GCP"

  gcloud config set project "$GCP_PROJECT_ID"

  local current_account
  current_account="$(gcloud auth list --filter=status:ACTIVE --format='value(account)' | head -n 1 || true)"

  if [[ -z "$current_account" ]]; then
    echo "Nenhuma conta gcloud autenticada." >&2
    echo "Executa: gcloud auth login" >&2
    exit 1
  fi

  echo "Conta gcloud ativa: ${current_account}"
}

enable_required_apis() {
  log "Ativar APIs obrigatórias"

  gcloud services enable \
    serviceusage.googleapis.com \
    container.googleapis.com \
    artifactregistry.googleapis.com \
    iamcredentials.googleapis.com \
    compute.googleapis.com \
    cloudbuild.googleapis.com \
    storage.googleapis.com \
    --project "$GCP_PROJECT_ID"
}

ensure_cloud_build_permissions() {
  log "Garantir permissões do Cloud Build"

  local project_number
  local cloud_build_sa

  project_number="$(gcloud projects describe "$GCP_PROJECT_ID" --format='value(projectNumber)')"
  cloud_build_sa="${project_number}@cloudbuild.gserviceaccount.com"

  echo "Cloud Build Service Account: ${cloud_build_sa}"

  gcloud projects add-iam-policy-binding "$GCP_PROJECT_ID" \
    --member="serviceAccount:${cloud_build_sa}" \
    --role="roles/artifactregistry.writer" \
    --quiet >/dev/null 2>&1 || true

  echo "Permissão artifactregistry.writer garantida."
}

ensure_artifact_registry() {
  log "Garantir Artifact Registry"

  if gcloud artifacts repositories describe "$ARTIFACT_REGISTRY_REPOSITORY" \
    --location="$ARTIFACT_REGISTRY_LOCATION" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    echo "Artifact Registry já existe."
  else
    echo "A criar Artifact Registry..."

    gcloud artifacts repositories create "$ARTIFACT_REGISTRY_REPOSITORY" \
      --repository-format=docker \
      --location="$ARTIFACT_REGISTRY_LOCATION" \
      --description="BookCloud container images" \
      --project="$GCP_PROJECT_ID"
  fi
}

validate_local_environment
print_cd_config

configure_gcp
enable_required_apis
ensure_cloud_build_permissions
ensure_artifact_registry

log "GCP, APIs, permissões e Artifact Registry configurados com sucesso"