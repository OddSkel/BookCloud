#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

BASE_URL="${BASE_URL:?BASE_URL is required}"
API_BASE_URL="${API_BASE_URL:-${BASE_URL}/api}"

export BASE_URL
export API_BASE_URL

if [[ -n "${KEYCLOAK_ACCESS_TOKEN:-}" ]]; then
  export KEYCLOAK_ACCESS_TOKEN
  export AUTHORIZATION_HEADER="${AUTHORIZATION_HEADER:-Bearer ${KEYCLOAK_ACCESS_TOKEN}}"
fi

request() {
  local url="$1"

  if [[ -n "${AUTHORIZATION_HEADER:-}" ]]; then
    curl -fsS \
      -H "Authorization: ${AUTHORIZATION_HEADER}" \
      "$url"
  else
    curl -fsS "$url"
  fi
}

check_http_status() {
  local name="$1"
  local url="$2"
  local expected="${3:-200}"
  local response_file="/tmp/bookcloud-staging-response.json"

  printf 'Testing %-40s %s\n' "$name" "$url"

  if [[ -n "${AUTHORIZATION_HEADER:-}" ]]; then
    http_code="$(curl \
      --max-time 30 \
      --connect-timeout 10 \
      -sS \
      -o "$response_file" \
      -w "%{http_code}" \
      -H "Authorization: ${AUTHORIZATION_HEADER}" \
      "$url" || true)"
  else
    http_code="$(curl \
      --max-time 30 \
      --connect-timeout 10 \
      -sS \
      -o "$response_file" \
      -w "%{http_code}" \
      "$url" || true)"
  fi

  if [[ "$http_code" != "$expected" ]]; then
    printf 'Request failed for "%s". Expected HTTP %s, got HTTP %s\n' "$name" "$expected" "$http_code" >&2
    printf 'Response body:\n' >&2
    cat "$response_file" >&2 || true
    exit 1
  fi
}

export -f request
export -f check_http_status

printf 'Running staging tests against %s\n' "$API_BASE_URL"

check_http_status "api-gateway health" "${API_BASE_URL}/health"

"${SCRIPT_DIR}/integration.sh"
"${SCRIPT_DIR}/acceptance.sh"

printf '\nAll staging integration and acceptance tests passed.\n'
