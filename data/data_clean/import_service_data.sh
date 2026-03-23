#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

POSTGRES_USER_ARG="${POSTGRES_USER:-bookcloud}"
POSTGRES_PASSWORD_ARG="${POSTGRES_PASSWORD:-bookcloud}"
POSTGRES_DB_ARG="${POSTGRES_DB:-bookcloud_db}"
POSTGRES_HOST_ARG="${POSTGRES_HOST:-localhost}"
POSTGRES_PORT_ARG="${POSTGRES_PORT:-5432}"
POSTGRES_SCHEMA_ARG="${POSTGRES_SCHEMA:-public}"

SERVICE_ARG="all"
CSV_DIR_ARG="${CSV_DIR:-${SCRIPT_DIR}/normalized_out}"
CHUNKSIZE_ARG="${CHUNKSIZE:-200000}"
MAX_CHUNKS_PER_FILE_ARG="${MAX_CHUNKS_PER_FILE:-0}"
DROP_ALL_ARG="false"
PYTHON_BIN="${PYTHON_BIN:-python3}"

usage() {
  cat <<EOF
Uso:
  $(basename "$0") -S <service> [opções]

Serviços:
  bookCatalog
  ratingCatalog
  authorCatalog
  genreAnalysis
  all

Opções:
  -S, --service              Serviço a importar
  -u, --user                 POSTGRES_USER
  -p, --password             POSTGRES_PASSWORD
  -d, --database             POSTGRES_DB
  -h, --host                 POSTGRES_HOST
  -P, --port                 POSTGRES_PORT
  -s, --schema               POSTGRES_SCHEMA
  -c, --chunksize            Número de linhas por chunk
  -m, --max-chunks           Número máximo de chunks por ficheiro (0 = todos)
  --csv-dir                  Diretório dos CSVs normalizados
  --drop-all                 Dá DROP às tabelas do serviço atual e sai
  --python                   Binário Python a usar
  --help                     Mostra ajuda

Exemplos:
  $(basename "$0") -S ratingCatalog -u user -p password -d db -h rating-db -P 5432 -c 50000 -m 2
  $(basename "$0") -S bookCatalog --drop-all -u user -p password -d db -h book-db -P 5432
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -S|--service)
      SERVICE_ARG="$2"
      shift 2
      ;;
    -u|--user)
      POSTGRES_USER_ARG="$2"
      shift 2
      ;;
    -p|--password)
      POSTGRES_PASSWORD_ARG="$2"
      shift 2
      ;;
    -d|--database)
      POSTGRES_DB_ARG="$2"
      shift 2
      ;;
    -h|--host)
      POSTGRES_HOST_ARG="$2"
      shift 2
      ;;
    -P|--port)
      POSTGRES_PORT_ARG="$2"
      shift 2
      ;;
    -s|--schema)
      POSTGRES_SCHEMA_ARG="$2"
      shift 2
      ;;
    -c|--chunksize)
      CHUNKSIZE_ARG="$2"
      shift 2
      ;;
    -m|--max-chunks)
      MAX_CHUNKS_PER_FILE_ARG="$2"
      shift 2
      ;;
    --csv-dir)
      CSV_DIR_ARG="$2"
      shift 2
      ;;
    --drop-all)
      DROP_ALL_ARG="true"
      shift
      ;;
    --python)
      PYTHON_BIN="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Opção inválida: $1"
      echo
      usage
      exit 1
      ;;
  esac
done

export POSTGRES_USER="${POSTGRES_USER_ARG}"
export POSTGRES_PASSWORD="${POSTGRES_PASSWORD_ARG}"
export POSTGRES_DB="${POSTGRES_DB_ARG}"
export POSTGRES_HOST="${POSTGRES_HOST_ARG}"
export POSTGRES_PORT="${POSTGRES_PORT_ARG}"
export POSTGRES_SCHEMA="${POSTGRES_SCHEMA_ARG}"

# compatibilidade com o código python
export PGUSER="${POSTGRES_USER_ARG}"
export PGPASSWORD="${POSTGRES_PASSWORD_ARG}"
export PGDATABASE="${POSTGRES_DB_ARG}"
export PGHOST="${POSTGRES_HOST_ARG}"
export PGPORT="${POSTGRES_PORT_ARG}"
export PGSCHEMA="${POSTGRES_SCHEMA_ARG}"

export SERVICE="${SERVICE_ARG}"
export CSV_DIR="${CSV_DIR_ARG}"
export CHUNKSIZE="${CHUNKSIZE_ARG}"
export MAX_CHUNKS_PER_FILE="${MAX_CHUNKS_PER_FILE_ARG}"

echo "========================================"
echo "BookCloud import runner"
echo "SERVICE=${SERVICE}"
echo "POSTGRES_HOST=${POSTGRES_HOST}"
echo "POSTGRES_PORT=${POSTGRES_PORT}"
echo "POSTGRES_USER=${POSTGRES_USER}"
echo "POSTGRES_DB=${POSTGRES_DB}"
echo "POSTGRES_SCHEMA=${POSTGRES_SCHEMA}"
echo "CSV_DIR=${CSV_DIR}"
echo "CHUNKSIZE=${CHUNKSIZE}"
echo "MAX_CHUNKS_PER_FILE=${MAX_CHUNKS_PER_FILE}"
echo "DROP_ALL=${DROP_ALL_ARG}"
echo "========================================"

if [[ "${DROP_ALL_ARG}" == "true" ]]; then
  exec "${PYTHON_BIN}" "${SCRIPT_DIR}/index.py" -S "${SERVICE_ARG}" --drop-all
else
  exec "${PYTHON_BIN}" "${SCRIPT_DIR}/index.py" -S "${SERVICE_ARG}"
fi