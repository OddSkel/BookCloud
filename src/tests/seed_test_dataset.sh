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

run_sql() {
  name="$1"
  container="$2"
  db_user="$3"
  db_name="$4"
  statement="$5"

  printf "Ensuring schema for %s...\n" "$name"

  docker exec -i "$container" \
    psql -U "$db_user" -d "$db_name" -v ON_ERROR_STOP=1 <<SQL
${statement}
SQL
}

ensure_book_schema() {
  run_sql "book-catalog" "$BOOK_DB_CONTAINER" "$BOOK_DB_USER" "$BOOK_DB_NAME" "
CREATE TABLE IF NOT EXISTS book (
  isbn BIGINT PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT,
  summary_clean TEXT,
  pub_year INTEGER
);
"
}

ensure_author_schema() {
  run_sql "author-catalog" "$AUTHOR_DB_CONTAINER" "$AUTHOR_DB_USER" "$AUTHOR_DB_NAME" "
CREATE TABLE IF NOT EXISTS author (
  author_id BIGINT PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS book_author (
  book_isbn BIGINT NOT NULL,
  author_id BIGINT NOT NULL,
  PRIMARY KEY (book_isbn, author_id)
);
"
}

ensure_genre_schema() {
  run_sql "genre-analysis" "$GENRE_DB_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_NAME" "
CREATE TABLE IF NOT EXISTS genre (
  genre_id INTEGER PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS book_genre (
  book_isbn BIGINT NOT NULL,
  genre_id INTEGER NOT NULL,
  PRIMARY KEY (book_isbn, genre_id)
);

CREATE TABLE IF NOT EXISTS genre_stats_cache (
  genre_id INTEGER PRIMARY KEY,
  avg_rating DOUBLE PRECISION DEFAULT 0,
  total_num_ratings BIGINT DEFAULT 0,
  book_count INTEGER DEFAULT 0,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS genre_year_stats_cache (
  genre_id INTEGER NOT NULL,
  year INTEGER NOT NULL,
  avg_rating DOUBLE PRECISION DEFAULT 0,
  total_num_ratings BIGINT DEFAULT 0,
  book_count INTEGER DEFAULT 0,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (genre_id, year)
);

DO \$\$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_stats_cache' AND column_name = 'average_rating'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_stats_cache' AND column_name = 'avg_rating'
  ) THEN
    ALTER TABLE genre_stats_cache RENAME COLUMN average_rating TO avg_rating;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_stats_cache' AND column_name = 'total_ratings'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_stats_cache' AND column_name = 'total_num_ratings'
  ) THEN
    ALTER TABLE genre_stats_cache RENAME COLUMN total_ratings TO total_num_ratings;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_stats_cache' AND column_name = 'total_books'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_stats_cache' AND column_name = 'book_count'
  ) THEN
    ALTER TABLE genre_stats_cache RENAME COLUMN total_books TO book_count;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'pub_year'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'year'
  ) THEN
    ALTER TABLE genre_year_stats_cache RENAME COLUMN pub_year TO year;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'average_rating'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'avg_rating'
  ) THEN
    ALTER TABLE genre_year_stats_cache RENAME COLUMN average_rating TO avg_rating;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'total_ratings'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'total_num_ratings'
  ) THEN
    ALTER TABLE genre_year_stats_cache RENAME COLUMN total_ratings TO total_num_ratings;
  END IF;

  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'total_books'
  ) AND NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'genre_year_stats_cache' AND column_name = 'book_count'
  ) THEN
    ALTER TABLE genre_year_stats_cache RENAME COLUMN total_books TO book_count;
  END IF;
END
\$\$;

ALTER TABLE genre_stats_cache
  ADD COLUMN IF NOT EXISTS avg_rating DOUBLE PRECISION DEFAULT 0,
  ADD COLUMN IF NOT EXISTS total_num_ratings BIGINT DEFAULT 0,
  ADD COLUMN IF NOT EXISTS book_count INTEGER DEFAULT 0,
  ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE genre_year_stats_cache
  ADD COLUMN IF NOT EXISTS year INTEGER,
  ADD COLUMN IF NOT EXISTS avg_rating DOUBLE PRECISION DEFAULT 0,
  ADD COLUMN IF NOT EXISTS total_num_ratings BIGINT DEFAULT 0,
  ADD COLUMN IF NOT EXISTS book_count INTEGER DEFAULT 0,
  ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;
"
}

ensure_rating_schema() {
  run_sql "rating-catalog" "$RATING_DB_CONTAINER" "$RATING_DB_USER" "$RATING_DB_NAME" "
CREATE TABLE IF NOT EXISTS rating (
  book_isbn BIGINT PRIMARY KEY,
  star_rating DOUBLE PRECISION,
  num_ratings BIGINT
);
"
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

ensure_author_schema
ensure_genre_schema
ensure_book_schema
ensure_rating_schema

if [ -x "${SCRIPT_DIR}/cleanup_test_dataset.sh" ]; then
  "${SCRIPT_DIR}/cleanup_test_dataset.sh" || true
fi

seed_csv "author-catalog" "$AUTHOR_DB_CONTAINER" "$AUTHOR_DB_USER" "$AUTHOR_DB_NAME" "author" "$AUTHOR_CSV"
seed_csv "genre-analysis" "$GENRE_DB_CONTAINER" "$GENRE_DB_USER" "$GENRE_DB_NAME" "genre" "$GENRE_CSV"
seed_csv "book-catalog" "$BOOK_DB_CONTAINER" "$BOOK_DB_USER" "$BOOK_DB_NAME" "book" "$BOOK_CSV"
seed_csv "rating-catalog" "$RATING_DB_CONTAINER" "$RATING_DB_USER" "$RATING_DB_NAME" "rating" "$RATING_CSV"

printf "Test dataset seeded successfully.\n"
