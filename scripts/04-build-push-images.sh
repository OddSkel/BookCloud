#!/usr/bin/env bash

set -euo pipefail

source "$(dirname "$0")/00-env.sh"

# ============================================================
# 4. Build/push das imagens
# ============================================================

services=(
  "api-gateway|src/api-gateway|src/api-gateway/Dockerfile"
  "author-analytics-service|src/services/author-analytics-service|src/services/author-analytics-service/Dockerfile"
  "author-catalog|src/services/author-catalog|src/services/author-catalog/Dockerfile"
  "book-catalog|src/services/book-catalog|src/services/book-catalog/Dockerfile"
  "book-recommendation|src/services/book-recommendation|src/services/book-recommendation/Dockerfile"
  "book-search|.|src/services/book-search/Dockerfile"
  "compare-service|src/services/compare-service|src/services/compare-service/Dockerfile"
  "genre-analysis-service|src/services/genre-analysis-service|src/services/genre-analysis-service/Dockerfile"
  "rating-catalog|src/services/rating-catalog|src/services/rating-catalog/Dockerfile"
)

cloud_build_image() {
  local service="$1"
  local context="$2"
  local dockerfile="$3"

  local image="${IMAGE_PREFIX}/${service}"
  local config_file
  config_file="$(mktemp)"

  local dockerfile_for_cloudbuild="$dockerfile"

  if [[ "$context" != "." ]]; then
    dockerfile_for_cloudbuild="Dockerfile"
  fi

cat > "$config_file" <<EOF
steps:
  - name: gcr.io/cloud-builders/docker
    entrypoint: bash
    env:
      - DOCKER_BUILDKIT=1
    args:
      - -c
      - |
        docker pull ${image}:main || true

        docker build \\
          --cache-from ${image}:main \\
          --build-arg BUILDKIT_INLINE_CACHE=1 \\
          -f ${dockerfile_for_cloudbuild} \\
          -t ${image}:${IMAGE_TAG} \\
          -t ${image}:main \\
          .

images:
  - ${image}:${IMAGE_TAG}
  - ${image}:main
EOF

  echo ""
  echo "============================================================"
  echo "Cloud Build: ${service}"
  echo "============================================================"
  echo "Context:    ${context}"
  echo "Dockerfile: ${dockerfile_for_cloudbuild}"
  echo "Image:      ${image}:${IMAGE_TAG}"
  echo "Image main: ${image}:main"

  echo "Cloud Build config gerado:"
  cat "$config_file"

  gcloud builds submit "$context" \
    --config "$config_file" \
    --project "$GCP_PROJECT_ID"

  rm -f "$config_file"
}

build_and_push_images() {
  if [[ "$SKIP_BUILD" == "true" ]]; then
    log "SKIP_BUILD=true. A saltar build/push das imagens"
    return 0
  fi

  log "Build e push das imagens via Cloud Build"

  for item in "${services[@]}"; do
    IFS="|" read -r service context dockerfile <<< "$item"

    require_file "$dockerfile"

    cloud_build_image "$service" "$context" "$dockerfile"
  done
}

validate_local_environment

build_and_push_images

log "Build e push das imagens concluídos com sucesso"
