#!/usr/bin/env sh

set -eu

BASE_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

NETWORK_NAME="bookcloud-network"

# =========================
# NETWORK
# =========================
if ! docker network inspect "$NETWORK_NAME" >/dev/null 2>&1; then
    printf 'Creating network %s...\n' "$NETWORK_NAME"
    docker network create "$NETWORK_NAME"
else
    printf 'Network %s already exists.\n' "$NETWORK_NAME"
fi

# =========================
# SERVICES
# =========================
for compose_file in "$BASE_DIR"/services/*/docker-compose.yml; do
    [ -f "$compose_file" ] || continue

    service_dir=$(dirname "$compose_file")
    service_name=$(basename "$service_dir")

    printf 'Starting %s...\n' "$service_name"
    (
        cd "$service_dir"
        docker compose up --build -d
    )
done

# =========================
# API GATEWAY
# =========================
if [ -f "$BASE_DIR/api-gateway/docker-compose.yml" ]; then
    printf 'Starting api-gateway...\n'
    (
        cd "$BASE_DIR/api-gateway"
        docker compose up --build -d
    )
fi