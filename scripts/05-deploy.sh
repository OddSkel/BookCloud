#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/00-env.sh"

# ============================================================
# 5. Deploy
# ============================================================
# - cria namespace
# - reserva/obtém IP fixo do Kong
# - renderiza manifests numa pasta temporária
# - ajusta namespace
# - troca imagens Docker
# - injeta IP fixo no Service do Kong
# - aplica manifests
# - espera deployments subirem
# - mostra URL pública
# ============================================================

services=(
  "api-gateway"
  "author-analytics-service"
  "author-catalog"
  "book-catalog"
  "book-recommendation"
  "book-search"
  "compare-service"
  "genre-analysis-service"
  "rating-catalog"
)

# ------------------------------------------------------------
# Reserva ou obtém o IP fixo usado pelo Service LoadBalancer do Kong
# ------------------------------------------------------------

reserve_static_ip() {
  local ip_name="$1"
  local ip_region="$2"

  log "Garantir IP fixo do Kong: ${ip_name}"

  if gcloud compute addresses describe "$ip_name" \
    --region "$ip_region" \
    --project "$GCP_PROJECT_ID" >/dev/null 2>&1; then

    echo "IP fixo já existe."
  else
    echo "A criar IP fixo..."

    gcloud compute addresses create "$ip_name" \
      --region "$ip_region" \
      --project "$GCP_PROJECT_ID"
  fi

  RESERVED_KONG_STATIC_IP="$(gcloud compute addresses describe "$ip_name" \
    --region "$ip_region" \
    --project "$GCP_PROJECT_ID" \
    --format="value(address)")"

  if [[ -z "$RESERVED_KONG_STATIC_IP" ]]; then
    echo "Não foi possível obter o IP fixo do Kong." >&2
    exit 1
  fi

  export RESERVED_KONG_STATIC_IP

  echo "Kong static IP: ${RESERVED_KONG_STATIC_IP}"
}

# ------------------------------------------------------------
# Ajusta os YAMLs para o namespace correto
# ------------------------------------------------------------

patch_rendered_namespace() {
  local rendered_dir="$1"
  local namespace="$2"

  echo "A ajustar manifests para namespace: ${namespace}"

  python3 - "$rendered_dir" "$namespace" <<'PY'
import sys
from pathlib import Path

rendered_dir = Path(sys.argv[1])
namespace = sys.argv[2]

yaml_files = list(rendered_dir.rglob("*.yaml")) + list(rendered_dir.rglob("*.yml"))

for path in yaml_files:
    lines = path.read_text().splitlines()
    out = []

    in_metadata = False
    is_namespace_file = path.name == "namespace.yaml"

    for line in lines:
        stripped = line.strip()

        # Ajusta kustomization.yaml:
        # namespace: bookcloud
        # namespace: bookcloud-test
        if path.name == "kustomization.yaml" and stripped.startswith("namespace:"):
            out.append(f"namespace: {namespace}")
            continue

        # Ajusta namespace.yaml:
        # metadata:
        #   name: bookcloud
        if is_namespace_file:
            if stripped == "metadata:":
                in_metadata = True
                out.append(line)
                continue

            if in_metadata and stripped.startswith("name:"):
                indent = line[:len(line) - len(line.lstrip())]
                out.append(f"{indent}name: {namespace}")
                in_metadata = False
                continue

        # Ajusta apenas linhas exatamente iguais a:
        # namespace: bookcloud
        #
        # Importante:
        # Não altera namespace: bookcloud-test,
        # evitando gerar bookcloud-test-test.
        if stripped == "namespace: bookcloud":
            indent = line[:len(line) - len(line.lstrip())]
            out.append(f"{indent}namespace: {namespace}")
            continue

        out.append(line)

    path.write_text("\n".join(out) + "\n")
PY

  # Se o kustomization.yaml existir mas não tiver namespace, adiciona.
  if [[ -f "$rendered_dir/kustomization.yaml" ]]; then
    if ! grep -qE '^namespace:' "$rendered_dir/kustomization.yaml"; then
      printf '\nnamespace: %s\n' "$namespace" >> "$rendered_dir/kustomization.yaml"
    fi
  fi
}

# ------------------------------------------------------------
# Prepara uma cópia temporária dos manifests para deploy
# ------------------------------------------------------------

render_manifests_for_namespace() {
  local namespace="$1"
  local static_ip="$2"

  log "Renderizar manifests Kubernetes para ${namespace}"

  local rendered_dir
  rendered_dir="$(mktemp -d)"

  cp -R k8s "$rendered_dir/bookcloud-k8s"

  RENDERED_K8S_DIR="$rendered_dir/bookcloud-k8s"
  export RENDERED_K8S_DIR

  echo "Rendered dir: ${RENDERED_K8S_DIR}"

  patch_rendered_namespace "$RENDERED_K8S_DIR" "$namespace"

  log "Substituir imagens nos deployments renderizados"

  for service in "${services[@]}"; do
    local deployment_file="${RENDERED_K8S_DIR}/${service}/deployment.yaml"
    local image="${IMAGE_PREFIX}/${service}:${IMAGE_TAG}"

    if [[ ! -f "$deployment_file" ]]; then
      echo "Deployment file não encontrado: ${deployment_file}" >&2
      exit 1
    fi

    sed -i "s#image: bookcloud/${service}:latest#image: ${image}#g" "$deployment_file"

    if ! grep -F "image: ${image}" "$deployment_file" >/dev/null; then
      echo "Falha ao substituir imagem para ${service}" >&2
      echo "Verifica se o deployment contém exatamente: image: bookcloud/${service}:latest" >&2
      exit 1
    fi
  done

  find "$RENDERED_K8S_DIR" -name deployment.yaml \
    -exec sed -i 's/imagePullPolicy: IfNotPresent/imagePullPolicy: Always/g' {} +

  log "Injetar IP fixo no Service do Kong"

  local kong_service_file=""

  for candidate in \
    "$RENDERED_K8S_DIR/kong/service.yaml" \
    "$RENDERED_K8S_DIR/kong/kong-service.yaml"
  do
    if [[ -f "$candidate" ]]; then
      kong_service_file="$candidate"
      break
    fi
  done

  if [[ -z "$kong_service_file" ]]; then
    echo "Service do Kong não encontrado." >&2
    echo "Esperado: k8s/kong/service.yaml ou k8s/kong/kong-service.yaml" >&2
    exit 1
  fi

  echo "Kong service file: ${kong_service_file}"

  if grep -F "KONG_STATIC_IP_PLACEHOLDER" "$kong_service_file" >/dev/null; then
    sed -i "s#KONG_STATIC_IP_PLACEHOLDER#${static_ip}#g" "$kong_service_file"
  elif grep -E "^[[:space:]]*loadBalancerIP:" "$kong_service_file" >/dev/null; then
    sed -i "s#^[[:space:]]*loadBalancerIP:.*#  loadBalancerIP: ${static_ip}#g" "$kong_service_file"
  else
    sed -i "/^[[:space:]]*type:[[:space:]]*LoadBalancer/a\\  loadBalancerIP: ${static_ip}" "$kong_service_file"
  fi

  echo "Service do Kong após patch:"
  grep -A 20 -n "type: LoadBalancer" "$kong_service_file" || true

  log "Debug namespace nos manifests renderizados"

  echo "Namespaces encontrados:"
  grep -R "namespace:" "$RENDERED_K8S_DIR" || true

  echo ""
  echo "Namespace resource:"
  if [[ -f "$RENDERED_K8S_DIR/namespace.yaml" ]]; then
    cat "$RENDERED_K8S_DIR/namespace.yaml"
  fi
}

# ------------------------------------------------------------
# Aguarda rollout dos deployments
# ------------------------------------------------------------

wait_rollout_or_debug() {
  local namespace="$1"
  local deployment="$2"

  echo ""
  echo "A aguardar rollout: ${deployment}"

  if ! kubectl -n "$namespace" rollout status "deployment/${deployment}" --timeout="$ROLLOUT_TIMEOUT"; then
    echo "Rollout falhou para ${deployment}" >&2

    echo ""
    echo "=== Deployment describe: ${deployment} ==="
    kubectl -n "$namespace" describe deployment "$deployment" || true

    echo ""
    echo "=== Pods ==="
    kubectl -n "$namespace" get pods -o wide || true

    echo ""
    echo "=== Pods describe: app=${deployment} ==="
    kubectl -n "$namespace" describe pods -l app="$deployment" || true

    echo ""
    echo "=== Logs: ${deployment} ==="
    kubectl -n "$namespace" logs "deployment/${deployment}" --tail=200 || true

    exit 1
  fi
}

# ------------------------------------------------------------
# Resolve IP público do Kong
# ------------------------------------------------------------

resolve_public_url() {
  local namespace="$1"

  log "Resolver URL pública do Kong"

  local external_ip=""

  for i in {1..60}; do
    external_ip="$(kubectl -n "$namespace" get svc kong \
      -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || true)"

    if [[ -n "$external_ip" ]]; then
      PUBLIC_BASE_URL="http://${external_ip}"
      export PUBLIC_BASE_URL

      echo "PUBLIC_BASE_URL=${PUBLIC_BASE_URL}"
      return 0
    fi

    echo "Kong ainda sem IP público. Tentativa ${i}/60..."
    kubectl -n "$namespace" get svc kong -o wide || true
    sleep 10
  done

  echo "Não foi possível resolver o IP público do Kong a tempo." >&2
  exit 1
}

# ------------------------------------------------------------
# Deploy principal
# ------------------------------------------------------------

deploy_environment() {
  local namespace="$1"
  local kong_ip_name="$2"
  local environment_label="$3"

  log "Iniciar deploy do ambiente ${environment_label}"

  echo "Namespace:    ${namespace}"
  echo "Kong IP name: ${kong_ip_name}"
  echo "Image prefix: ${IMAGE_PREFIX}"
  echo "Image tag:    ${IMAGE_TAG}"

  log "Criar namespace, se não existir"

  kubectl create namespace "$namespace" \
    --dry-run=client \
    -o yaml | kubectl apply --validate=false -f -

  reserve_static_ip "$kong_ip_name" "$KONG_STATIC_IP_REGION"

  render_manifests_for_namespace "$namespace" "$RESERVED_KONG_STATIC_IP"

  log "Validar manifests com dry-run server"

  kubectl apply -k "$RENDERED_K8S_DIR" --dry-run=server

  log "Aplicar manifests Kubernetes"

  kubectl apply -k "$RENDERED_K8S_DIR"

  log "Esperar rollouts principais"

  deployments_to_wait=(
    "kong"
    "api-gateway"
    "author-analytics-service"
    "author-catalog"
    "book-catalog"
    "book-recommendation"
    "book-search"
    "compare-service"
    "genre-analysis-service"
    "rating-catalog"
  )

  for deployment in "${deployments_to_wait[@]}"; do
    wait_rollout_or_debug "$namespace" "$deployment"
  done

  log "Estado final do namespace"

  kubectl -n "$namespace" get pods -o wide
  kubectl -n "$namespace" get svc -o wide
  kubectl -n "$namespace" get hpa -o wide || true

  resolve_public_url "$namespace"

  log "Deploy do ambiente ${environment_label} concluído com sucesso"

  echo "NAMESPACE=${namespace}"
  echo "KONG_STATIC_IP=${RESERVED_KONG_STATIC_IP}"
  echo "PUBLIC_BASE_URL=${PUBLIC_BASE_URL}"
  echo "IMAGE_TAG=${IMAGE_TAG}"
}

# ============================================================
# Execução
# ============================================================

validate_local_environment
print_cd_config

echo ""
echo "Este script vai aplicar manifests Kubernetes no namespace:"
echo "${K8S_NAMESPACE}"
echo ""

read -r -p "Continuar com o deploy? Escreve YES: " confirmation

if [[ "$confirmation" != "YES" ]]; then
  echo "Operação cancelada."
  exit 0
fi

deploy_environment \
  "$K8S_NAMESPACE" \
  "$KONG_STATIC_IP_NAME" \
  "current"

log "Deploy concluído com sucesso"
