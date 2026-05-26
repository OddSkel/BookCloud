#!/usr/bin/env sh

set -eu

BASE_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

compose_down() {
    service_dir="$1"

    (
        cd "$service_dir"

        if [ -f ".env" ]; then
            docker compose --env-file .env down
        else
            docker compose down
        fi
    )
}

# =========================
# SERVICES
# =========================
for compose_file in "$BASE_DIR"/services/*/docker-compose.yml; do
    [ -f "$compose_file" ] || continue

    service_dir=$(dirname "$compose_file")
    service_name=$(basename "$service_dir")

    printf 'Stopping %s...\n' "$service_name"
    compose_down "$service_dir"
done

# =========================
# API GATEWAY
# =========================
if [ -f "$BASE_DIR/api-gateway/docker-compose.yml" ]; then
    printf 'Stopping api-gateway...\n'
    compose_down "$BASE_DIR/api-gateway"
fi

printf 'All services stopped.\n'