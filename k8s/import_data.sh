#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

NAMESPACE="${BOOKCLOUD_NAMESPACE:-bookcloud}"
DB_USER="${BOOKCLOUD_DB_USER:-bookcloud}"
DB_PASSWORD="${BOOKCLOUD_DB_PASSWORD:-bookcloud-pass}"
CSV_DIR="${BOOKCLOUD_CSV_DIR:-$REPO_ROOT/data/data_clean/normalized_out}"
CHUNK_SIZE="${BOOKCLOUD_IMPORT_CHUNK_SIZE:-10000}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
IMPORTER="$REPO_ROOT/data/data_clean/db/import_service_csv.py"
DROP_BEFORE_IMPORT="${BOOKCLOUD_DROP_BEFORE_IMPORT:-0}"

BOOK_PORT="${BOOKCLOUD_BOOK_DB_PORT:-15432}"
AUTHOR_PORT="${BOOKCLOUD_AUTHOR_DB_PORT:-15433}"
RATING_PORT="${BOOKCLOUD_RATING_DB_PORT:-15434}"
GENRE_PORT="${BOOKCLOUD_GENRE_DB_PORT:-15435}"

PORT_FORWARD_PIDS=()

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Error: command '$1' not found." >&2
    exit 1
  fi
}

cleanup() {
  local pid

  for pid in "${PORT_FORWARD_PIDS[@]}"; do
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
  done
}

run() {
  echo "+ $*"
  "$@"
}

start_port_forward() {
  local service="$1"
  local local_port="$2"
  local log_file

  log_file="$(mktemp)"
  echo "+ kubectl -n $NAMESPACE port-forward svc/$service $local_port:5432"
  kubectl -n "$NAMESPACE" port-forward "svc/$service" "$local_port:5432" >"$log_file" 2>&1 &
  PORT_FORWARD_PIDS+=("$!")

  for _ in $(seq 1 30); do
    if grep -q "Forwarding from" "$log_file"; then
      rm -f "$log_file"
      return 0
    fi

    if ! kill -0 "${PORT_FORWARD_PIDS[-1]}" >/dev/null 2>&1; then
      cat "$log_file" >&2
      rm -f "$log_file"
      echo "Error: failed to start port-forward for service '$service'." >&2
      exit 1
    fi

    sleep 1
  done

  cat "$log_file" >&2
  rm -f "$log_file"
  echo "Error: timed out waiting for port-forward for service '$service'." >&2
  exit 1
}

import_service() {
  local service="$1"
  local port="$2"
  local database="$3"
  local drop_flag=()

  if [[ "$DROP_BEFORE_IMPORT" == "1" ]]; then
    drop_flag+=(--drop-before-import)
  fi

  run "$PYTHON_BIN" "$IMPORTER" \
    --service "$service" \
    --csv-dir "$CSV_DIR" \
    --host 127.0.0.1 \
    --port "$port" \
    --user "$DB_USER" \
    --password "$DB_PASSWORD" \
    --database "$database" \
    --schema public \
    --chunk-size "$CHUNK_SIZE" \
    --create-tables \
    "${drop_flag[@]}"
}

trap cleanup EXIT

require_command kubectl
require_command "$PYTHON_BIN"

if [[ ! -d "$CSV_DIR" ]]; then
  echo "Error: CSV directory not found: $CSV_DIR" >&2
  exit 1
fi

if [[ ! -f "$IMPORTER" ]]; then
  echo "Error: importer not found: $IMPORTER" >&2
  exit 1
fi

run kubectl -n "$NAMESPACE" get svc book-db author-db rating-db genre-db >/dev/null

start_port_forward book-db "$BOOK_PORT"
start_port_forward author-db "$AUTHOR_PORT"
start_port_forward rating-db "$RATING_PORT"
start_port_forward genre-db "$GENRE_PORT"

import_service book "$BOOK_PORT" book_catalog
import_service author "$AUTHOR_PORT" author_catalog
import_service rating "$RATING_PORT" rating_catalog
import_service genre "$GENRE_PORT" genre_analysis

cat <<EOF

Data import finished.

Imported services:
  - book    -> book_catalog
  - author  -> author_catalog
  - rating  -> rating_catalog
  - genre   -> genre_analysis

EOF
