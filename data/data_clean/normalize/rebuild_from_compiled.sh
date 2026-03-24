#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

INPUT_FILE="${1:-/home/gusta/Documents/mei/cn/BookCloud/data/dataset/compiled_books.csv}"
MAX_ROWS="${2:-0}"
CHUNKSIZE="${3:-100000}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
OUTPUT_DIR="/home/gusta/Documents/mei/cn/BookCloud/data/data_clean/normalized_out"

echo "========================================"
echo "REBUILD NORMALIZED OUTPUTS FROM COMPILED"
echo "========================================"
echo "INPUT_FILE=${INPUT_FILE}"
echo "OUTPUT_DIR=${OUTPUT_DIR}"
echo "MAX_ROWS=${MAX_ROWS}"
echo "CHUNKSIZE=${CHUNKSIZE}"
echo "========================================"

rm -rf "${OUTPUT_DIR}"
mkdir -p "${OUTPUT_DIR}"

exec "${PYTHON_BIN}" "${BASE_DIR}/create_normalized_from_compiled.py" \
  --input "${INPUT_FILE}" \
  --output "${OUTPUT_DIR}" \
  --chunksize "${CHUNKSIZE}" \
  --max-rows "${MAX_ROWS}"
