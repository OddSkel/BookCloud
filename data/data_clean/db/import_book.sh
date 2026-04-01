#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_BIN="${PYTHON_BIN:-python3}"
CSV_DIR="${CSV_DIR:-../normalized_out}"
HOST="${HOST:-localhost}"
SCHEMA="${SCHEMA:-public}"

DROP_FLAG=""
if [[ "${1:-}" == "-D" ]]; then
  DROP_FLAG="--drop-before-import"
  shift
fi

if [[ $# -lt 4 ]]; then
  echo "Usage: $0 [-D] <USER> <PASSWORD> <PORT> <DATABASE> [CHUNK_SIZE]"
  exit 1
fi

USER_NAME="$1"
PASSWORD="$2"
PORT="$3"
DATABASE="$4"
CHUNK_SIZE="${5:-10000}"

exec "${PYTHON_BIN}" "${BASE_DIR}/import_service_csv.py" \
  --service book \
  --csv-dir "${CSV_DIR}" \
  --host "${HOST}" \
  --port "${PORT}" \
  --user "${USER_NAME}" \
  --password "${PASSWORD}" \
  --database "${DATABASE}" \
  --schema "${SCHEMA}" \
  --chunk-size "${CHUNK_SIZE}" \
  --create-tables \
  ${DROP_FLAG}
