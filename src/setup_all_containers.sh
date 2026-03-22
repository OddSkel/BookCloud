#!/usr/bin/env sh

set -eu

BASE_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

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
