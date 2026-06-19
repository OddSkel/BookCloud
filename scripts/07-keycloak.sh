#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# BookCloud CD - 7. Keycloak/Kong JWT
# ============================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ -f "$SCRIPT_DIR/00-env.sh" ]]; then
  # shellcheck source=/dev/null
  source "$SCRIPT_DIR/00-env.sh"
elif [[ -f "$REPO_ROOT/scripts/00-env.sh" ]]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/scripts/00-env.sh"
elif [[ -f "$REPO_ROOT/00-env.sh" ]]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/00-env.sh"
else
  echo "Erro: não encontrei 00-env.sh." >&2
  echo "Esperado em: $SCRIPT_DIR/00-env.sh ou $REPO_ROOT/scripts/00-env.sh" >&2
  exit 1
fi

NAMESPACE="${NAMESPACE:-${K8S_NAMESPACE:-bookcloud-test}}"
K8S_NAMESPACE="${K8S_NAMESPACE:-$NAMESPACE}"
ROLLOUT_TIMEOUT="${ROLLOUT_TIMEOUT:-600s}"

KEYCLOAK_SERVICE="${KEYCLOAK_SERVICE:-keycloak}"
KEYCLOAK_SERVICE_PORT="${KEYCLOAK_SERVICE_PORT:-80}"
LOCAL_KEYCLOAK_PORT="${LOCAL_KEYCLOAK_PORT:-18080}"
KEYCLOAK_URL="${KEYCLOAK_URL:-http://127.0.0.1:${LOCAL_KEYCLOAK_PORT}}"

REALM="${REALM:-${KEYCLOAK_REALM:-bookcloud}}"
KEYCLOAK_REALM_CANDIDATES="${KEYCLOAK_REALM_CANDIDATES:-bookcloud bookcloud-realm}"

KEYCLOAK_ADMIN_USERNAME="${KEYCLOAK_ADMIN_USERNAME:-admin}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-bookcloud-pass}"
KEYCLOAK_CLIENT_ID="${KEYCLOAK_CLIENT_ID:-bookcloud-app}"
KEYCLOAK_CLIENT_SECRET="${KEYCLOAK_CLIENT_SECRET:-bookcloud-app-secret}"
KEYCLOAK_TEST_PASSWORD="${KEYCLOAK_TEST_PASSWORD:-password}"
KEYCLOAK_TEST_USERS="${KEYCLOAK_TEST_USERS:-testuser:user adminuser:admin readonlyuser:readonly}"
KEYCLOAK_REALM_ROLES="${KEYCLOAK_REALM_ROLES:-user admin readonly}"
KEYCLOAK_REALM_IMPORT_FILE="${KEYCLOAK_REALM_IMPORT_FILE:-bookcloud-realm.json}"

KONG_CONFIGMAP_FILE="${KONG_CONFIGMAP_FILE:-k8s/kong/configmap.yaml}"
if [[ "$KONG_CONFIGMAP_FILE" != /* ]]; then
  KONG_CONFIGMAP_FILE="$REPO_ROOT/$KONG_CONFIGMAP_FILE"
fi

if [[ "$KEYCLOAK_REALM_IMPORT_FILE" != /* ]]; then
  KEYCLOAK_REALM_IMPORT_FILE="$REPO_ROOT/$KEYCLOAK_REALM_IMPORT_FILE"
fi

PF_PID=""
KEYCLOAK_POD=""
PUBKEY_PEM=""

cleanup() {
  if [[ -n "${PF_PID:-}" ]]; then
    kill "$PF_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_command_local() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Comando obrigatório não encontrado: $1" >&2
    exit 1
  fi
}

validate_keycloak_environment() {
  log "Validar ambiente Keycloak/Kong"

  require_command_local kubectl
  require_command_local curl
  require_command_local jq
  require_command_local openssl
  require_command_local base64
  require_command_local awk

  if [[ ! -f "$KONG_CONFIGMAP_FILE" ]]; then
    echo "Erro: ConfigMap do Kong não encontrado: $KONG_CONFIGMAP_FILE" >&2
    exit 1
  fi

  if ! kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
    echo "Erro: namespace não existe ou kubectl não tem acesso: $NAMESPACE" >&2
    exit 1
  fi

  if ! kubectl -n "$NAMESPACE" get svc "$KEYCLOAK_SERVICE" >/dev/null 2>&1; then
    echo "Erro: Service do Keycloak não encontrado: svc/$KEYCLOAK_SERVICE no namespace $NAMESPACE" >&2
    kubectl -n "$NAMESPACE" get svc >&2 || true
    exit 1
  fi

  if ! kubectl -n "$NAMESPACE" get deployment kong >/dev/null 2>&1; then
    echo "Erro: deployment/kong não encontrado no namespace $NAMESPACE" >&2
    exit 1
  fi

  KEYCLOAK_POD="$(kubectl -n "$NAMESPACE" get pod -l app=keycloak -o jsonpath='{.items[0].metadata.name}')"
  if [[ -z "$KEYCLOAK_POD" ]]; then
    echo "Erro: pod Keycloak não encontrado com label app=keycloak no namespace $NAMESPACE" >&2
    exit 1
  fi

  echo "Keycloak pod: $KEYCLOAK_POD"
}

start_keycloak_port_forward() {
  log "Abrir port-forward para Keycloak"

  if curl -sf "$KEYCLOAK_URL/realms/master" >/dev/null 2>&1; then
    echo "Keycloak já acessível em: $KEYCLOAK_URL"
    return 0
  fi

  echo "Namespace: $NAMESPACE"
  echo "Service:   $KEYCLOAK_SERVICE"
  echo "Porta:     localhost:${LOCAL_KEYCLOAK_PORT} -> svc/${KEYCLOAK_SERVICE}:${KEYCLOAK_SERVICE_PORT}"

  kubectl -n "$NAMESPACE" port-forward \
    "svc/${KEYCLOAK_SERVICE}" \
    "${LOCAL_KEYCLOAK_PORT}:${KEYCLOAK_SERVICE_PORT}" \
    >/tmp/bookcloud-keycloak-port-forward.log 2>&1 &

  PF_PID="$!"

  for i in {1..60}; do
    if curl -sf "$KEYCLOAK_URL/realms/master" >/dev/null 2>&1; then
      echo "Keycloak acessível em: $KEYCLOAK_URL"
      return 0
    fi

    if ! kill -0 "$PF_PID" >/dev/null 2>&1; then
      echo "Erro: port-forward terminou antes de Keycloak ficar acessível." >&2
      cat /tmp/bookcloud-keycloak-port-forward.log >&2 || true
      exit 1
    fi

    echo "A aguardar Keycloak... tentativa ${i}/60"
    sleep 3
  done

  echo "Erro: Keycloak não ficou acessível a tempo." >&2
  cat /tmp/bookcloud-keycloak-port-forward.log >&2 || true
  exit 1
}

resolve_keycloak_admin_password_from_k8s() {
  local secret_name=""
  local secret_key=""
  local secret_value=""

  secret_name="$(kubectl -n "$NAMESPACE" get deployment keycloak \
    -o jsonpath='{.spec.template.spec.containers[0].env[?(@.name=="KEYCLOAK_ADMIN_PASSWORD")].valueFrom.secretKeyRef.name}' \
    2>/dev/null || true)"

  secret_key="$(kubectl -n "$NAMESPACE" get deployment keycloak \
    -o jsonpath='{.spec.template.spec.containers[0].env[?(@.name=="KEYCLOAK_ADMIN_PASSWORD")].valueFrom.secretKeyRef.key}' \
    2>/dev/null || true)"

  if [[ -z "$secret_name" || -z "$secret_key" ]]; then
    echo "Aviso: não encontrei secretKeyRef de KEYCLOAK_ADMIN_PASSWORD no deployment/keycloak."
    echo "A usar KEYCLOAK_ADMIN_PASSWORD recebido por variável de ambiente/default."
    return 0
  fi

  if ! kubectl -n "$NAMESPACE" get secret "$secret_name" >/dev/null 2>&1; then
    echo "Aviso: Kubernetes secret não encontrado: ${secret_name}."
    echo "A usar KEYCLOAK_ADMIN_PASSWORD recebido por variável de ambiente/default."
    return 0
  fi

  secret_value="$(kubectl -n "$NAMESPACE" get secret "$secret_name" \
    -o jsonpath="{.data.${secret_key}}" 2>/dev/null | base64 -d 2>/dev/null || true)"

  if [[ -z "$secret_value" ]]; then
    echo "Aviso: não consegui ler ${secret_name}/${secret_key}."
    echo "A usar KEYCLOAK_ADMIN_PASSWORD recebido por variável de ambiente/default."
    return 0
  fi

  KEYCLOAK_ADMIN_PASSWORD="$secret_value"
  export KEYCLOAK_ADMIN_PASSWORD

  echo "KEYCLOAK_ADMIN_PASSWORD resolvida a partir do Kubernetes secret: ${secret_name}/${secret_key}"
}

kcadm_login() {
  log "Autenticar Keycloak Admin CLI"

  resolve_keycloak_admin_password_from_k8s

  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh config credentials \
    --server http://localhost:8080 \
    --realm master \
    --user "$KEYCLOAK_ADMIN_USERNAME" \
    --password "$KEYCLOAK_ADMIN_PASSWORD"

  echo "kcadm autenticado como $KEYCLOAK_ADMIN_USERNAME."
}

realm_exists() {
  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get "realms/$1" >/dev/null 2>&1
}

ensure_realm() {
  log "Garantir realm Keycloak"

  if realm_exists "$REALM"; then
    echo "Realm já existe: $REALM"
    return 0
  fi

  echo "Realm não existe: $REALM"

  if [[ -f "$KEYCLOAK_REALM_IMPORT_FILE" ]]; then
    echo "A importar realm a partir de: $KEYCLOAK_REALM_IMPORT_FILE"
    kubectl -n "$NAMESPACE" cp "$KEYCLOAK_REALM_IMPORT_FILE" "$KEYCLOAK_POD:/tmp/bookcloud-realm.json"
    kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh create realms \
      -f /tmp/bookcloud-realm.json

    if realm_exists "$REALM"; then
      echo "Realm importado: $REALM"
      return 0
    fi

    echo "Erro: importação terminou, mas o realm esperado não existe: $REALM" >&2
    echo "Confirma se o campo .realm do JSON é igual a REALM=$REALM." >&2
    exit 1
  fi

  echo "Ficheiro de import não encontrado. A criar realm mínimo: $REALM"
  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh create realms \
    -s realm="$REALM" \
    -s enabled=true \
    -s defaultSignatureAlgorithm=RS256
}

ensure_realm_roles() {
  log "Garantir roles do realm"

  local role
  for role in $KEYCLOAK_REALM_ROLES; do
    if kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get "roles/$role" -r "$REALM" >/dev/null 2>&1; then
      echo "Role já existe: $role"
    else
      kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh create roles \
        -r "$REALM" \
        -s name="$role"
      echo "Role criada: $role"
    fi
  done
}

ensure_client() {
  log "Garantir client Keycloak"

  local client_uuid
  client_uuid="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get clients \
    -r "$REALM" \
    -q clientId="$KEYCLOAK_CLIENT_ID" \
    | jq -r '.[0].id // empty')"

  if [[ -z "$client_uuid" ]]; then
    echo "Client não existe. A criar: $KEYCLOAK_CLIENT_ID"
    kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh create clients \
      -r "$REALM" \
      -s clientId="$KEYCLOAK_CLIENT_ID" \
      -s enabled=true \
      -s publicClient=false \
      -s serviceAccountsEnabled=true \
      -s directAccessGrantsEnabled=true \
      -s standardFlowEnabled=true \
      -s protocol=openid-connect \
      -s clientAuthenticatorType=client-secret

    client_uuid="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get clients \
      -r "$REALM" \
      -q clientId="$KEYCLOAK_CLIENT_ID" \
      | jq -r '.[0].id')"
  fi

  echo "Client UUID: $client_uuid"

  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh update "clients/$client_uuid" \
    -r "$REALM" \
    -s clientId="$KEYCLOAK_CLIENT_ID" \
    -s enabled=true \
    -s publicClient=false \
    -s serviceAccountsEnabled=true \
    -s directAccessGrantsEnabled=true \
    -s standardFlowEnabled=true \
    -s protocol=openid-connect \
    -s clientAuthenticatorType=client-secret

  # Em algumas versões do Keycloak, atualizar o campo secret no client não é suficiente.
  # Este endpoint força o secret esperado.
  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh update "clients/$client_uuid/client-secret" \
    -r "$REALM" \
    -s value="$KEYCLOAK_CLIENT_SECRET" >/dev/null 2>&1 || \
  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh update "clients/$client_uuid" \
    -r "$REALM" \
    -s secret="$KEYCLOAK_CLIENT_SECRET"

  echo "Client configurado: $KEYCLOAK_CLIENT_ID"
}

ensure_service_account_roles() {
  log "Garantir permissões da service account do client"

  local client_uuid service_account_id realm_mgmt_uuid role role_json

  client_uuid="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get clients \
    -r "$REALM" \
    -q clientId="$KEYCLOAK_CLIENT_ID" \
    | jq -r '.[0].id')"

  service_account_id="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get "clients/$client_uuid/service-account-user" \
    -r "$REALM" | jq -r '.id')"

  realm_mgmt_uuid="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get clients \
    -r "$REALM" \
    -q clientId=realm-management \
    | jq -r '.[0].id')"

  for role in manage-users view-users query-users view-realm; do
    role_json="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get "clients/$realm_mgmt_uuid/roles/$role" \
      -r "$REALM")"

    printf '%s' "[$role_json]" | kubectl -n "$NAMESPACE" exec -i "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh create \
      "users/$service_account_id/role-mappings/clients/$realm_mgmt_uuid" \
      -r "$REALM" \
      -f - >/dev/null 2>&1 || true

    echo "✓ service account role: $role"
  done
}

create_or_update_user() {
  local username="$1"
  local role="$2"
  local user_id

  echo "Creating/updating user: $username -> $role"

  user_id="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get users \
    -r "$REALM" \
    -q username="$username" \
    -q exact=true \
    | jq -r '.[0].id // empty')"

  if [[ -z "$user_id" ]]; then
    kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh create users \
      -r "$REALM" \
      -s username="$username" \
      -s enabled=true

    user_id="$(kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get users \
      -r "$REALM" \
      -q username="$username" \
      -q exact=true \
      | jq -r '.[0].id')"
  fi

  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh set-password \
    -r "$REALM" \
    --username "$username" \
    --new-password "$KEYCLOAK_TEST_PASSWORD"

  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh add-roles \
    -r "$REALM" \
    --uusername "$username" \
    --rolename "$role" >/dev/null 2>&1 || true

  echo "OK: $username -> $role"
}

ensure_test_users() {
  log "Garantir utilizadores de teste"

  local pair username role
  for pair in $KEYCLOAK_TEST_USERS; do
    username="${pair%%:*}"
    role="${pair##*:}"
    create_or_update_user "$username" "$role"
  done

  echo "Utilizadores atuais:"
  kubectl -n "$NAMESPACE" exec "$KEYCLOAK_POD" -- /opt/keycloak/bin/kcadm.sh get users \
    -r "$REALM" \
    --fields username,enabled \
    | jq .
}

validate_direct_login() {
  log "Validar login direto no Keycloak"

  local admin_user token_response access_token
  admin_user="adminuser"

  token_response="$(curl -sf -X POST \
    "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=password" \
    -d "client_id=$KEYCLOAK_CLIENT_ID" \
    -d "client_secret=$KEYCLOAK_CLIENT_SECRET" \
    -d "username=$admin_user" \
    -d "password=$KEYCLOAK_TEST_PASSWORD")" || {
      echo "Erro: login direto no Keycloak falhou." >&2
      exit 1
    }

  access_token="$(echo "$token_response" | jq -r '.access_token // empty')"
  if [[ -z "$access_token" || "$access_token" == "null" ]]; then
    echo "Erro: Keycloak não devolveu access_token." >&2
    echo "$token_response" | jq . >&2 || echo "$token_response" >&2
    exit 1
  fi

  echo "Login direto OK. Token começa por: $(echo "$access_token" | cut -c1-20)..."
}

fetch_keycloak_public_key() {
  log "Obter chave pública RS256 do Keycloak"

  local jwks_url="$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/certs"
  local jwks x5c

  jwks="$(curl -sf "$jwks_url")" || {
    echo "Erro: falha ao obter JWKS: $jwks_url" >&2
    exit 1
  }

  if ! echo "$jwks" | jq -e '.keys' >/dev/null 2>&1; then
    echo "Erro: resposta JWKS inválida." >&2
    echo "$jwks" >&2
    exit 1
  fi

  x5c="$(echo "$jwks" | jq -r '.keys[]? | select(.alg=="RS256" and .use=="sig") | .x5c[0] // empty' | head -n 1)"

  if [[ -z "$x5c" || "$x5c" == "null" ]]; then
    echo "Erro: não foi encontrado certificado x5c RS256 de assinatura no JWKS." >&2
    echo "$jwks" | jq . >&2
    exit 1
  fi

  PUBKEY_PEM="$(echo "$x5c" | base64 -d | openssl x509 -inform DER -pubkey -noout 2>/dev/null)"

  if [[ -z "$PUBKEY_PEM" ]]; then
    echo "Erro: não foi possível extrair a public key PEM." >&2
    exit 1
  fi

  echo "Chave pública extraída com sucesso."
}

patch_kong_declarative_config() {
  log "Aplicar ConfigMap do Kong com issuer e public key reais"

  local issuer="http://keycloak/realms/$REALM"
  local rendered
  rendered="$(mktemp)"

  echo "Namespace:       $NAMESPACE"
  echo "Realm:           $REALM"
  echo "Issuer esperado: $issuer"
  echo "ConfigMap file:  $KONG_CONFIGMAP_FILE"

  export AWK_NAMESPACE="$NAMESPACE"
  export AWK_ISSUER="$issuer"
  export AWK_PEM="$PUBKEY_PEM"

  awk '
    /^  namespace:/ {
      print "  namespace: " ENVIRON["AWK_NAMESPACE"]
      next
    }

    /KONG_JWT_ISSUER_PLACEHOLDER/ {
      sub(/KONG_JWT_ISSUER_PLACEHOLDER/, ENVIRON["AWK_ISSUER"])
    }

    /KONG_JWT_PUBLIC_KEY_PLACEHOLDER/ {
      indent = substr($0, 1, match($0, /[^ ]/) - 1)
      n = split(ENVIRON["AWK_PEM"], lines, "\n")
      for (i = 1; i <= n; i++) {
        if (lines[i] != "") print indent lines[i]
      }
      next
    }

    { print }
  ' "$KONG_CONFIGMAP_FILE" > "$rendered"

  if grep -q 'KONG_JWT_PUBLIC_KEY_PLACEHOLDER\|KONG_JWT_ISSUER_PLACEHOLDER' "$rendered"; then
    echo "Erro: placeholders ainda existem no ConfigMap renderizado." >&2
    grep -n 'KONG_JWT_PUBLIC_KEY_PLACEHOLDER\|KONG_JWT_ISSUER_PLACEHOLDER' "$rendered" >&2 || true
    rm -f "$rendered"
    exit 1
  fi

  if ! grep -q -- '-----BEGIN PUBLIC KEY-----' "$rendered"; then
    echo "Erro: public key PEM não ficou presente no ConfigMap renderizado." >&2
    rm -f "$rendered"
    exit 1
  fi

  kubectl apply --dry-run=server -f "$rendered"
  kubectl apply -f "$rendered"

  rm -f "$rendered"
}

restart_kong() {
  log "Reiniciar Kong"

  kubectl -n "$NAMESPACE" rollout restart deployment/kong
  kubectl -n "$NAMESPACE" rollout status deployment/kong --timeout="$ROLLOUT_TIMEOUT"

  kubectl -n "$NAMESPACE" get pods -l app=kong
  kubectl -n "$NAMESPACE" logs deployment/kong --tail=80 || true
}

main() {
  print_cd_config || true
  validate_keycloak_environment
  start_keycloak_port_forward
  kcadm_login
  ensure_realm
  ensure_realm_roles
  ensure_client
  ensure_service_account_roles
  ensure_test_users
  validate_direct_login
  fetch_keycloak_public_key
  patch_kong_declarative_config
  restart_kong

  log "Keycloak/Kong configurados com sucesso"
  echo "NAMESPACE=$NAMESPACE"
  echo "REALM=$REALM"
  echo "ISSUER=http://keycloak/realms/$REALM"
  echo "CLIENT_ID=$KEYCLOAK_CLIENT_ID"
  echo "TEST_USERS=$KEYCLOAK_TEST_USERS"
}

main "$@"
