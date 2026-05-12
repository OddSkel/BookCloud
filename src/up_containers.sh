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

start_service() {
    service_dir="$1"
    service_name=$(basename "$service_dir")

    # =========================
    # ENV FILE
    # =========================
    if [ -f "$service_dir/.example.env" ]; then
        printf 'Copying .example.env to .env for %s...\n' "$service_name"
        cp "$service_dir/.example.env" "$service_dir/.env"
    else
        printf 'No .example.env found for %s. Skipping env copy.\n' "$service_name"
    fi

    printf 'Starting %s...\n' "$service_name"
    (
        cd "$service_dir"
        docker compose up --build -d
    )
}

# =========================
# SERVICES - CATALOG FIRST
# =========================
for compose_file in "$BASE_DIR"/services/*-catalog/docker-compose.yml; do
    [ -f "$compose_file" ] || continue

    service_dir=$(dirname "$compose_file")
    start_service "$service_dir"
done

# =========================
# SERVICES - REST
# =========================
for compose_file in "$BASE_DIR"/services/*/docker-compose.yml; do
    [ -f "$compose_file" ] || continue

    service_dir=$(dirname "$compose_file")
    service_name=$(basename "$service_dir")

    case "$service_name" in
        *-catalog)
            continue
            ;;
    esac

    start_service "$service_dir"
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

# =========================
# KONG GATEWAY
# =========================
if [ -f "$BASE_DIR/kong/docker-compose.yml" ]; then
    printf 'Starting kong...\n'
    (
        cd "$BASE_DIR/kong"
        docker compose up --build -d
    )
fi

printf 'All services started.\n'