#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/00-env.sh"

# ============================================================
# BookCloud - Drop All
# ============================================================

export MONITORING_NAMESPACE="${MONITORING_NAMESPACE:-monitoring}"

export DELETE_NAMESPACES="${DELETE_NAMESPACES:-true}"
export DELETE_MONITORING="${DELETE_MONITORING:-true}"
export DELETE_STATIC_IPS="${DELETE_STATIC_IPS:-true}"
export DELETE_GKE_CLUSTER="${DELETE_GKE_CLUSTER:-true}"

export DELETE_ARTIFACT_REGISTRY="${DELETE_ARTIFACT_REGISTRY:-false}"
export DELETE_DATASET_BUCKET="${DELETE_DATASET_BUCKET:-false}"

confirm_or_exit() {
  echo ""
  echo "ATENÇÃO: Isto pode apagar recursos reais no Google Cloud."
  echo ""
  echo "Projeto GCP:              ${GCP_PROJECT_ID}"
  echo "Cluster GKE:              ${GKE_CLUSTER}"
  echo "Localização GKE:          ${GKE_LOCATION}"
  echo "Namespace produção:       ${PROD_NAMESPACE}"
  echo "Namespace teste:          ${TEST_NAMESPACE}"
  echo "Namespace monitoring:     ${MONITORING_NAMESPACE}"
  echo "IP produção Kong:         ${PROD_KONG_STATIC_IP_NAME}"
  echo "IP teste Kong:            ${TEST_KONG_STATIC_IP_NAME}"
  echo "Artifact Registry:        ${ARTIFACT_REGISTRY_REPOSITORY}"
  echo "Dataset bucket:           gs://${DATASET_BUCKET}"
  echo ""
  echo "DELETE_NAMESPACES:        ${DELETE_NAMESPACES}"
  echo "DELETE_MONITORING:        ${DELETE_MONITORING}"
  echo "DELETE_STATIC_IPS:        ${DELETE_STATIC_IPS}"
  echo "DELETE_GKE_CLUSTER:       ${DELETE_GKE_CLUSTER}"
  echo "DELETE_ARTIFACT_REGISTRY: ${DELETE_ARTIFACT_REGISTRY}"
  echo "DELETE_DATASET_BUCKET:    ${DELETE_DATASET_BUCKET}"
  echo ""

  read -r -p "Para continuar, escreve DROP: " confirmation

  if [[ "$confirmation" != "DROP" ]]; then
    echo "Operação cancelada."
    exit 0
  fi

  read -r -p "Confirma novamente escrevendo o nome do projeto (${GCP_PROJECT_ID}): " project_confirmation

  if [[ "$project_confirmation" != "$GCP_PROJECT_ID" ]]; then
    echo "Projeto não confirmado corretamente. Operação cancelada."
    exit 0
  fi
}

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

get_gke_credentials_if_cluster_exists() {
  log "Obter credenciais do GKE, se o cluster existir"

  if gcloud container clusters describe "$GKE_CLUSTER" \
    --location="$GKE_LOCATION" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    gcloud container clusters get-credentials "$GKE_CLUSTER" \
      --location="$GKE_LOCATION" \
      --project="$GCP_PROJECT_ID"

    echo "Credenciais obtidas."
  else
    echo "Cluster não existe. A saltar get-credentials."
  fi
}

delete_namespace_if_exists() {
  local namespace="$1"

  if kubectl get namespace "$namespace" >/dev/null 2>&1; then
    echo "A apagar namespace: ${namespace}"
    kubectl delete namespace "$namespace" --wait=false || true
  else
    echo "Namespace não existe: ${namespace}"
  fi
}

wait_namespace_deleted() {
  local namespace="$1"

  echo "A aguardar remoção do namespace: ${namespace}"

  for i in {1..60}; do
    if ! kubectl get namespace "$namespace" >/dev/null 2>&1; then
      echo "Namespace removido: ${namespace}"
      return 0
    fi

    echo "Namespace ainda existe: ${namespace}. Tentativa ${i}/60..."
    sleep 5
  done

  echo "Namespace ${namespace} ainda existe. Pode estar preso por finalizers." >&2
}

delete_kubernetes_namespaces() {
  if [[ "$DELETE_NAMESPACES" != "true" ]]; then
    echo "DELETE_NAMESPACES=false. A saltar namespaces da aplicação."
    return 0
  fi

  log "Apagar namespaces da aplicação"

  if ! gcloud container clusters describe "$GKE_CLUSTER" \
    --location="$GKE_LOCATION" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    echo "Cluster não existe. A saltar namespaces."
    return 0
  fi

  delete_namespace_if_exists "$TEST_NAMESPACE"
  delete_namespace_if_exists "$PROD_NAMESPACE"

  wait_namespace_deleted "$TEST_NAMESPACE"
  wait_namespace_deleted "$PROD_NAMESPACE"
}

delete_monitoring_namespace() {
  if [[ "$DELETE_MONITORING" != "true" ]]; then
    echo "DELETE_MONITORING=false. A saltar monitoring."
    return 0
  fi

  log "Apagar namespace de monitoring"

  if ! gcloud container clusters describe "$GKE_CLUSTER" \
    --location="$GKE_LOCATION" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    echo "Cluster não existe. A saltar monitoring."
    return 0
  fi

  delete_namespace_if_exists "$MONITORING_NAMESPACE"
  wait_namespace_deleted "$MONITORING_NAMESPACE"
}

delete_gke_cluster() {
  if [[ "$DELETE_GKE_CLUSTER" != "true" ]]; then
    echo "DELETE_GKE_CLUSTER=false. A saltar cluster GKE."
    return 0
  fi

  log "Apagar cluster GKE"

  if gcloud container clusters describe "$GKE_CLUSTER" \
    --location="$GKE_LOCATION" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    gcloud container clusters delete "$GKE_CLUSTER" \
      --location="$GKE_LOCATION" \
      --project="$GCP_PROJECT_ID" \
      --quiet

    echo "Cluster apagado."
  else
    echo "Cluster não existe: ${GKE_CLUSTER}"
  fi
}

delete_static_ip_if_exists() {
  local ip_name="$1"
  local ip_region="$2"

  if gcloud compute addresses describe "$ip_name" \
    --region "$ip_region" \
    --project "$GCP_PROJECT_ID" >/dev/null 2>&1; then

    echo "A apagar IP fixo: ${ip_name}"

    gcloud compute addresses delete "$ip_name" \
      --region "$ip_region" \
      --project "$GCP_PROJECT_ID" \
      --quiet

    echo "IP apagado: ${ip_name}"
  else
    echo "IP fixo não existe: ${ip_name}"
  fi
}

delete_static_ips() {
  if [[ "$DELETE_STATIC_IPS" != "true" ]]; then
    echo "DELETE_STATIC_IPS=false. A saltar IPs fixos."
    return 0
  fi

  log "Apagar IPs fixos do Kong"

  delete_static_ip_if_exists "$TEST_KONG_STATIC_IP_NAME" "$KONG_STATIC_IP_REGION"
  delete_static_ip_if_exists "$PROD_KONG_STATIC_IP_NAME" "$KONG_STATIC_IP_REGION"
}

delete_artifact_registry() {
  if [[ "$DELETE_ARTIFACT_REGISTRY" != "true" ]]; then
    echo "DELETE_ARTIFACT_REGISTRY=false. A manter Artifact Registry."
    return 0
  fi

  log "Apagar Artifact Registry"

  if gcloud artifacts repositories describe "$ARTIFACT_REGISTRY_REPOSITORY" \
    --location="$ARTIFACT_REGISTRY_LOCATION" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    gcloud artifacts repositories delete "$ARTIFACT_REGISTRY_REPOSITORY" \
      --location="$ARTIFACT_REGISTRY_LOCATION" \
      --project="$GCP_PROJECT_ID" \
      --quiet

    echo "Artifact Registry apagado."
  else
    echo "Artifact Registry não existe: ${ARTIFACT_REGISTRY_REPOSITORY}"
  fi
}

delete_dataset_bucket() {
  if [[ "$DELETE_DATASET_BUCKET" != "true" ]]; then
    echo "DELETE_DATASET_BUCKET=false. A manter bucket do dataset."
    return 0
  fi

  log "Apagar bucket do dataset"

  if gcloud storage buckets describe "gs://${DATASET_BUCKET}" \
    --project="$GCP_PROJECT_ID" >/dev/null 2>&1; then

    gcloud storage rm -r "gs://${DATASET_BUCKET}/**" \
      --project="$GCP_PROJECT_ID" || true

    gcloud storage buckets delete "gs://${DATASET_BUCKET}" \
      --project="$GCP_PROJECT_ID" \
      --quiet

    echo "Bucket apagado."
  else
    echo "Bucket não existe: gs://${DATASET_BUCKET}"
  fi
}

show_remaining_resources() {
  log "Resumo de recursos restantes"

  echo ""
  echo "Clusters GKE:"
  gcloud container clusters list \
    --project="$GCP_PROJECT_ID" || true

  echo ""
  echo "IPs reservados na região ${KONG_STATIC_IP_REGION}:"
  gcloud compute addresses list \
    --regions="$KONG_STATIC_IP_REGION" \
    --project="$GCP_PROJECT_ID" || true

  echo ""
  echo "Artifact Registry repositories:"
  gcloud artifacts repositories list \
    --location="$ARTIFACT_REGISTRY_LOCATION" \
    --project="$GCP_PROJECT_ID" || true

  echo ""
  echo "Buckets:"
  gcloud storage buckets list \
    --project="$GCP_PROJECT_ID" || true
}

require_command gcloud
require_command kubectl

configure_gcp
confirm_or_exit

get_gke_credentials_if_cluster_exists

delete_kubernetes_namespaces
delete_monitoring_namespace
delete_gke_cluster
delete_static_ips
delete_artifact_registry
delete_dataset_bucket

show_remaining_resources

log "Drop concluído"