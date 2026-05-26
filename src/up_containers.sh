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

ensure_env_file() {
    service_dir="$1"
    service_name=$(basename "$service_dir")

    if [ -f "$service_dir/.env" ]; then
        printf '.env already exists for %s. Keeping existing file.\n' "$service_name"
    elif [ -f "$service_dir/.example.env" ]; then
        printf 'Creating .env from .example.env for %s...\n' "$service_name"
        cp "$service_dir/.example.env" "$service_dir/.env"
    else
        printf 'No .env or .example.env found for %s.\n' "$service_name"
    fi
}

compose_cmd() {
    service_dir="$1"
    shift

    (
        cd "$service_dir"

        if [ -f ".env" ]; then
            docker compose --env-file .env "$@"
        else
            docker compose "$@"
        fi
    )
}

build_service() {
    service_dir="$1"
    service_name=$(basename "$service_dir")

    ensure_env_file "$service_dir"

    printf 'Building %s...\n' "$service_name"
    compose_cmd "$service_dir" build
}

start_service() {
    service_dir="$1"
    service_name=$(basename "$service_dir")

    ensure_env_file "$service_dir"

    printf 'Starting %s...\n' "$service_name"

    if [ "$COMPOSE_BUILD" = "true" ]; then
        compose_cmd "$service_dir" up -d --no-build
    else
        compose_cmd "$service_dir" up -d
    fi
}

# =========================
# COLLECT SERVICES
# =========================
SERVICE_DIRS=""

for compose_file in "$BASE_DIR"/services/*/docker-compose.yml; do
    [ -f "$compose_file" ] || continue
    SERVICE_DIRS="$SERVICE_DIRS $(dirname "$compose_file")"
done

if [ -f "$BASE_DIR/api-gateway/docker-compose.yml" ]; then
    SERVICE_DIRS="$SERVICE_DIRS $BASE_DIR/api-gateway"
fi

# =========================
# BUILD ALL SERVICES IN PARALLEL
# =========================
if [ "$COMPOSE_BUILD" = "true" ]; then
    printf 'Building all services in parallel...\n'

    BUILD_PIDS=""
    for service_dir in $SERVICE_DIRS; do
        build_service "$service_dir" &
        BUILD_PIDS="$BUILD_PIDS $!"
    done

    BUILD_FAILED=0
    for pid in $BUILD_PIDS; do
        if ! wait "$pid"; then
            BUILD_FAILED=1
        fi
    done

    if [ "$BUILD_FAILED" -ne 0 ]; then
        printf 'One or more service builds failed.\n' >&2
        exit 1
    fi

    printf 'All images built.\n'
fi

# =========================
# START CATALOG SERVICES FIRST
# =========================
for compose_file in "$BASE_DIR"/services/*-catalog/docker-compose.yml; do
    [ -f "$compose_file" ] || continue
    start_service "$(dirname "$compose_file")"
done

# =========================
# START OTHER SERVICES
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
# START API GATEWAY
# =========================
if [ -f "$BASE_DIR/api-gateway/docker-compose.yml" ]; then
    start_service "$BASE_DIR/api-gateway"
fi

printf 'All services started.\n'
