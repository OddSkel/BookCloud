#!/usr/bin/env sh

set -eu

SCRIPT_DIR="$(CDPATH= cd "$(dirname "$0")" && pwd -P)"
DATASET_DIR="${DATASET_DIR:-${SCRIPT_DIR}/dataset}"

BOOK_CSV="${DATASET_DIR}/book.csv"
AUTHOR_CSV="${DATASET_DIR}/author.csv"
GENRE_CSV="${DATASET_DIR}/genre.csv"
RATING_CSV="${DATASET_DIR}/rating.csv"

BOOK_DB_CONTAINER="${BOOK_DB_CONTAINER:-book-db}"
AUTHOR_DB_CONTAINER="${AUTHOR_DB_CONTAINER:-author-db}"
GENRE_DB_CONTAINER="${GENRE_DB_CONTAINER:-genre-db}"
RATING_DB_CONTAINER="${RATING_DB_CONTAINER:-rating-db}"

BOOK_TABLE="${BOOK_TABLE:-book}"
AUTHOR_TABLE="${AUTHOR_TABLE:-author}"
GENRE_TABLE="${GENRE_TABLE:-genre}"
RATING_TABLE="${RATING_TABLE:-rating}"

TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-120}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-5}"

container_env() {
  container="$1"
  key="$2"

  docker exec "$container" sh -c "printenv $key" 2>/dev/null || true
}

BOOK_DB_NAME="${BOOK_DB_NAME:-$(container_env "$BOOK_DB_CONTAINER" POSTGRES_DB)}"
AUTHOR_DB_NAME="${AUTHOR_DB_NAME:-$(container_env "$AUTHOR_DB_CONTAINER" POSTGRES_DB)}"
GENRE_DB_NAME="${GENRE_DB_NAME:-$(container_env "$GENRE_DB_CONTAINER" POSTGRES_DB)}"
RATING_DB_NAME="${RATING_DB_NAME:-$(container_env "$RATING_DB_CONTAINER" POSTGRES_DB)}"

BOOK_DB_USER="${BOOK_DB_USER:-$(container_env "$BOOK_DB_CONTAINER" POSTGRES_USER)}"
AUTHOR_DB_USER="${AUTHOR_DB_USER:-$(container_env "$AUTHOR_DB_CONTAINER" POSTGRES_USER)}"
GENRE_DB_USER="${GENRE_DB_USER:-$(container_env "$GENRE_DB_CONTAINER" POSTGRES_USER)}"
RATING_DB_USER="${RATING_DB_USER:-$(container_env "$RATING_DB_CONTAINER" POSTGRES_USER)}"

require_file() {
  if [ ! -f "$1" ]; then
    printf "Required dataset file not found: %s\n" "$1" >&2
    exit 1
  fi
}

wait_for_postgres() {
  name="$1"
  container="$2"
  db_user="$3"
  db_name="$4"

  printf "Waiting for %s database container %s...\n" "$name" "$container"

  elapsed=0

  while [ "$elapsed" -lt "$TIMEOUT_SECONDS" ]; do
    if docker exec "$container" pg_isready -U "$db_user" -d "$db_name" >/dev/null 2>&1; then
      printf "%s database is ready.\n" "$name"
      return 0
    fi

    sleep "$INTERVAL_SECONDS"
    elapsed=$((elapsed + INTERVAL_SECONDS))
  done

  printf "%s database did not become ready in %s seconds.\n" "$name" "$TIMEOUT_SECONDS" >&2
  docker ps -a >&2
  return 1
}

seed_csv() {
  name="$1"
  container="$2"
  db_user="$3"
  db_name="$4"
  table="$5"
  csv_file="$6"

  printf "Seeding %s from %s into %s.%s...\n" "$name" "$csv_file" "$db_name" "$table"

  cat "$csv_file" | docker exec -i "$container" \
    psql -U "$db_user" -d "$db_name" -v ON_ERROR_STOP=1 \
    -c "\\copy ${table} FROM STDIN WITH (FORMAT csv, HEADER true);"

  printf "%s seeded successfully.\n" "$name"
}

require_file "$BOOK_CSV"
require_file "$AUTHOR_CSV"
require_file "$GENRE_CSV"
require_file "$RATING_CSV"

printf "Using dataset from %s\n" "$DATASET_DIR"

wait_for_postgres "author-catalog" "$AUTHOR_DB_CONTAINER" "$AUTHOR_DB_USER" "$AUTHOR_DB_NAME"
wait_for_postgres "genre-analysis" "$GENRE_DB_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_NAME"
wait_for_postgres "book-catalog" "$BOOK_DB_CONTAINER" "$BOOK_DB_USER" "$BOOK_DB_NAME"
wait_for_postgres "rating-catalog" "$RATING_DB_CONTAINER" "$RATING_DB_USER" "$RATING_DB_NAME"

# Remove dados antigos antes de inserir para evitar duplicados.
if [ -x "${SCRIPT_DIR}/cleanup_test_dataset.sh" ]; then
  "${SCRIPT_DIR}/cleanup_test_dataset.sh" || true
fi

# Ordem importante
seed_csv "author-catalog" "$AUTHOR_DB_CONTAINER" "$AUTHOR_DB_USER" "$AUTHOR_DB_NAME" "$AUTHOR_TABLE" "$AUTHOR_CSV"
seed_csv "genre-analysis" "$GENRE_DB_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_NAME" "$GENRE_TABLE" "$GENRE_CSV"
seed_csv "book-catalog" "$BOOK_DB_CONTAINER" "$BOOK_DB_USER" "$BOOK_DB_NAME" "$BOOK_TABLE" "$BOOK_CSV"
seed_csv "rating-catalog" "$RATING_DB_CONTAINER" "$RATING_DB_USER" "$RATING_DB_NAME" "$RATING_TABLE" "$RATING_CSV"

printf "Test dataset seeded successfully.\n"