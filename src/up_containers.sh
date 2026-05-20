#!/usr/bin/env sh

set -eu

BASE_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

NETWORK_NAME="bookcloud-network"
COMPOSE_BUILD="true"

# =========================
# ARGUMENTS
# =========================
for arg in "$@"; do
    case "$arg" in
        --no-build)
            COMPOSE_BUILD="false"
            ;;
        --build)
            COMPOSE_BUILD="true"
            ;;
        *)
            printf 'Unknown option: %s\n' "$arg"
            printf 'Usage: %s [--build|--no-build]\n' "$0"
            exit 1
            ;;
    esac
done

printf 'Compose build enabled: %s\n' "$COMPOSE_BUILD"

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
        if [ "$COMPOSE_BUILD" = "true" ]; then
            docker compose up --build -d
        else
            docker compose up -d
        fi
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
    if [ -f "$BASE_DIR/api-gateway/.example.env" ]; then
        printf 'Copying .example.env to .env for api-gateway...\n'
        cp "$BASE_DIR/api-gateway/.example.env" "$BASE_DIR/api-gateway/.env"
    else
        printf 'No .example.env found for api-gateway. Skipping env copy.\n'
    fi

    printf 'Starting api-gateway...\n'
    (
        cd "$BASE_DIR/api-gateway"
        if [ "$COMPOSE_BUILD" = "true" ]; then
            docker compose up --build -d
        else
            docker compose up -d
        fi
    )
fi

printf 'All services started.\n'