#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Defaults relativos ao projeto
DEFAULT_INPUT="${BASE_DIR}/../data/dataset/compiled_books.csv"
DEFAULT_OUTPUT="${BASE_DIR}/../data/data_clean/normalized_out"

INPUT_FILE="${1:-${INPUT_FILE:-$DEFAULT_INPUT}}"
OUTPUT_DIR="${OUTPUT_DIR:-$DEFAULT_OUTPUT}"
MAX_ROWS="${2:-${MAX_ROWS:-0}}"
CHUNKSIZE="${3:-${CHUNKSIZE:-100000}}"
PYTHON_BIN="${PYTHON_BIN:-python3}"

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
