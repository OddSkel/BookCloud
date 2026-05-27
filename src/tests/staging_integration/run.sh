#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

BASE_URL="${BASE_URL:?BASE_URL is required}"
BASE_URL="${BASE_URL%/}"
API_BASE_URL="${API_BASE_URL:-${BASE_URL}/api}"
API_BASE_URL="${API_BASE_URL%/}"

export BASE_URL
export API_BASE_URL

if [[ -n "${KEYCLOAK_ACCESS_TOKEN:-}" ]]; then
  export KEYCLOAK_ACCESS_TOKEN
fi

if [[ -n "${KEYCLOAK_ACCESS_TOKEN:-}" && -z "${AUTHORIZATION_HEADER:-}" ]]; then
  export AUTHORIZATION_HEADER="Bearer ${KEYCLOAK_ACCESS_TOKEN}"
fi

printf 'Running staging tests against %s\n' "$API_BASE_URL"

if [[ -n "${AUTHORIZATION_HEADER:-}" ]]; then
  printf 'Authorization header is configured.\n'
else
  printf 'Authorization header is not configured. Requests will be sent without token.\n'
fi

"${SCRIPT_DIR}/integration.sh"
"${SCRIPT_DIR}/acceptance.sh"

printf '\nAll staging integration and acceptance tests passed.\n'
