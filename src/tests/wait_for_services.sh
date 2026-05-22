#!/usr/bin/env sh

set -eu

BASE_URL="${BASE_URL:-http://localhost:8080}"
API_BASE_URL="${API_BASE_URL:-${BASE_URL}/api}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-120}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-5}"

wait_for_url() {
    service_name="$1"
    url="$2"

    printf 'Waiting for %s at %s...\n' "$service_name" "$url"

    elapsed=0

    while [ "$elapsed" -lt "$TIMEOUT_SECONDS" ]; do
        if curl -fsS "$url" >/dev/null 2>&1; then
            printf '%s is ready.\n' "$service_name"
            return 0
        fi

        printf '%s is not ready yet. Retrying in %s seconds...\n' "$service_name" "$INTERVAL_SECONDS"

        sleep "$INTERVAL_SECONDS"
        elapsed=$((elapsed + INTERVAL_SECONDS))
    done

    printf 'ERROR: %s did not become ready after %s seconds.\n' "$service_name" "$TIMEOUT_SECONDS"
    printf 'Last checked URL: %s\n' "$url"

    return 1
}

wait_for_url "API Gateway" "$API_BASE_URL/health"

printf 'All required services are ready.\n'
