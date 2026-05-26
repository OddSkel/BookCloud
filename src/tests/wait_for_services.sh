#!/usr/bin/env sh

set -eu

BASE_URL="${BASE_URL:-http://localhost:8080}"
API_BASE_URL="${API_BASE_URL:-${BASE_URL}/api}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-120}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-5}"

wait_for_http() {
  name="$1"
  url="$2"

  printf 'Waiting for %s at %s...\n' "$name" "$url"

  elapsed=0

  while [ "$elapsed" -lt "$TIMEOUT_SECONDS" ]; do
    if curl -fsS "$url" >/dev/null 2>&1; then
      printf '%s is ready.\n' "$name"
      return 0
    fi

    sleep "$INTERVAL_SECONDS"
    elapsed=$((elapsed + INTERVAL_SECONDS))
  done

  printf '%s did not become ready in %s seconds.\n' "$name" "$TIMEOUT_SECONDS" >&2
  return 1
}

wait_for_http "API Gateway" "$API_BASE_URL/health"

printf 'All required services are ready.\n'
