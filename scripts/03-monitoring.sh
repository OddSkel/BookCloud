#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/00-env.sh"

# ============================================================
# 3. Instalar monitoring
# ============================================================

install_monitoring_stack() {
  log "Instalar monitoring stack com Helm"

  local monitoring_namespace="monitoring"
  local monitoring_release_name="monitoring"
  local monitoring_values_file="k8s/monitoring/values-gke.yaml"

  if command -v helm >/dev/null 2>&1; then
    echo "Helm disponível."
  else
    echo "Helm não encontrado. A instalar Helm..."

    curl -fsSL https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
  fi

  kubectl create namespace "$monitoring_namespace" \
    --dry-run=client \
    -o yaml | kubectl apply -f -

  helm repo add prometheus-community https://prometheus-community.github.io/helm-charts || true
  helm repo update

  if [[ -f "$monitoring_values_file" ]]; then
    helm upgrade --install "$monitoring_release_name" prometheus-community/kube-prometheus-stack \
      --namespace "$monitoring_namespace" \
      --values "$monitoring_values_file" \
      --wait \
      --timeout 10m
  else
    echo "Ficheiro de values não encontrado: ${monitoring_values_file}"
    echo "A instalar com valores default..."

    helm upgrade --install "$monitoring_release_name" prometheus-community/kube-prometheus-stack \
      --namespace "$monitoring_namespace" \
      --wait \
      --timeout 10m
  fi

  echo "A aguardar ServiceMonitor CRD..."

  for i in {1..60}; do
    if kubectl get crd servicemonitors.monitoring.coreos.com >/dev/null 2>&1; then
      echo "ServiceMonitor CRD disponível."
      return 0
    fi

    echo "ServiceMonitor CRD ainda não está pronto. Tentativa ${i}/60..."
    sleep 5
  done

  echo "ServiceMonitor CRD não ficou pronto a tempo." >&2
  exit 1
}

validate_local_environment

install_monitoring_stack

log "Monitoring instalado com sucesso"