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

first_csv_values_sql() {
  file="$1"
  column="$2"

  if [ ! -f "$file" ]; then
    printf "NULL"
    return 0
  fi

  values="$(awk -F ',' -v column="$column" '
    NR == 1 {
      for (i = 1; i <= NF; i++) {
        gsub(/\r$/, "", $i)
        if ($i == column) {
          column_index = i
          break
        }
      }
      next
    }

    column_index > 0 {
      value = $column_index
      gsub(/\r$/, "", value)
      gsub(/\047/, "\047\047", value)

      if (value != "") {
        printf "\047%s\047,", value
      }
    }
  ' "$file" | sed 's/,$//')"

  if [ -z "$values" ]; then
    printf "NULL"
  else
    printf "%s" "$values"
  fi
}

delete_by_column_in() {
  name="$1"
  container="$2"
  db_user="$3"
  db_name="$4"
  table="$5"
  column="$6"
  values="$7"

  if [ -z "$db_user" ] || [ -z "$db_name" ]; then
    printf "Skipping cleanup for %s because db user/name is empty.\n" "$name"
    return 0
  fi

  if [ "$values" = "NULL" ]; then
    printf "Skipping cleanup for %s because there are no values.\n" "$name"
    return 0
  fi

  if ! docker ps --format '{{.Names}}' | grep -Fx "$container" >/dev/null 2>&1; then
    printf "Skipping cleanup for %s because container does not exist: %s\n" "$name" "$container"
    return 0
  fi

  printf "Cleaning %s test data from %s where %s in (%s)...\n" "$name" "$table" "$column" "$values"

  docker exec "$container" \
    psql -U "$db_user" -d "$db_name" -v ON_ERROR_STOP=1 \
    -c "DELETE FROM ${table} WHERE ${column} IN (${values});"

  printf "%s cleanup completed.\n" "$name"
}

printf "Cleaning test dataset from databases...\n"

BOOK_ISBNS="$(first_csv_values_sql "$BOOK_CSV" isbn)"
AUTHOR_IDS="$(first_csv_values_sql "$AUTHOR_CSV" author_id)"
GENRE_IDS="$(first_csv_values_sql "$GENRE_CSV" genre_id)"
RATING_ISBNS="$(first_csv_values_sql "$RATING_CSV" book_isbn)"

# Ordem inversa do seed
delete_by_column_in "rating-catalog" "$RATING_DB_CONTAINER" "$RATING_DB_USER" "$RATING_DB_NAME" "$RATING_TABLE" "book_isbn" "$RATING_ISBNS"
delete_by_column_in "book-catalog" "$BOOK_DB_CONTAINER" "$BOOK_DB_USER" "$BOOK_DB_NAME" "$BOOK_TABLE" "isbn" "$BOOK_ISBNS"
delete_by_column_in "genre-analysis" "$GENRE_DB_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_NAME" "$GENRE_TABLE" "genre_id" "$GENRE_IDS"
delete_by_column_in "author-catalog" "$AUTHOR_DB_CONTAINER" "$AUTHOR_DB_USER" "$AUTHOR_DB_NAME" "$AUTHOR_TABLE" "author_id" "$AUTHOR_IDS"

printf "Test dataset cleanup completed.\n"