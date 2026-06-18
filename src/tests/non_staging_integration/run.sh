#!/usr/bin/env sh

set -eu

BASE_URL="${BASE_URL:-http://localhost:8080}"
API_BASE_URL="${API_BASE_URL:-${BASE_URL}/api}"

SCRIPT_DIR="$(CDPATH= cd "$(dirname "$0")" && pwd -P)"
DATASET_DIR="${DATASET_DIR:-${SCRIPT_DIR}/../dataset}"

BOOK_CSV="${DATASET_DIR}/book.csv"
AUTHOR_CSV="${DATASET_DIR}/author.csv"
GENRE_CSV="${DATASET_DIR}/genre.csv"
RATING_CSV="${DATASET_DIR}/rating.csv"

require_file() {
  if [ ! -f "$1" ]; then
    printf 'Required test dataset file not found: %s\n' "$1" >&2
    printf 'Current directory: %s\n' "$(pwd)" >&2
    printf 'Script directory: %s\n' "$SCRIPT_DIR" >&2
    printf 'Dataset directory: %s\n' "$DATASET_DIR" >&2

    if [ -d "$SCRIPT_DIR/.." ]; then
      printf 'Available files in tests directory:\n' >&2
      ls -la "$SCRIPT_DIR/.." >&2
    fi

    if [ -d "$DATASET_DIR" ]; then
      printf 'Available files in dataset directory:\n' >&2
      ls -la "$DATASET_DIR" >&2
    fi

    exit 1
  fi
}

first_csv_value() {
  file="$1"
  column="$2"

  awk -F ',' -v column="$column" '
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

      if (value != "") {
        print value
        exit
      }
    }
  ' "$file"
}

check_endpoint() {
  name="$1"
  url="$2"

  printf 'Testing %-34s %s\n' "$name" "$url"
  curl -fsS "$url" \
    -H "Authorization: Bearer $TOKEN" \
    >/tmp/bookcloud-integration-response.json
}

check_endpoint_contains() {
  name="$1"
  url="$2"
  expected="$3"

  check_endpoint "$name" "$url"

  if ! grep -F "$expected" /tmp/bookcloud-integration-response.json >/dev/null; then
    printf 'Expected response for "%s" to contain "%s".\n' "$name" "$expected" >&2
    printf 'Response body:\n' >&2
    cat /tmp/bookcloud-integration-response.json >&2
    exit 1
  fi
}

require_file "$BOOK_CSV"
require_file "$AUTHOR_CSV"
require_file "$GENRE_CSV"
require_file "$RATING_CSV"

BOOK_ISBN="$(first_csv_value "$BOOK_CSV" isbn)"
AUTHOR_ID="$(first_csv_value "$AUTHOR_CSV" author_id)"
GENRE_ID="$(first_csv_value "$GENRE_CSV" genre_id)"
GENRE_NAME="$(first_csv_value "$GENRE_CSV" name)"
RATING_ISBN="$(first_csv_value "$RATING_CSV" book_isbn)"

if [ -z "$BOOK_ISBN" ] || [ -z "$AUTHOR_ID" ] || [ -z "$GENRE_ID" ] || [ -z "$RATING_ISBN" ]; then
  printf 'Test dataset is incomplete. Check files in %s\n' "$DATASET_DIR" >&2
  exit 1
fi

trap 'rm -f /tmp/bookcloud-integration-response.json' EXIT INT TERM

TOKEN="eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJyZWFsbV9hY2Nlc3MiOnsicm9sZXMiOlsidXNlciJdfX0."

printf 'Running local integration tests against %s...\n' "$API_BASE_URL"
printf 'Using dataset from %s\n' "$DATASET_DIR"

check_endpoint "api-gateway health" \
  "$API_BASE_URL/health"

check_endpoint_contains "book-catalog get book" \
  "$API_BASE_URL/book/$BOOK_ISBN" \
  "$BOOK_ISBN"

check_endpoint_contains "author-catalog get author" \
  "$API_BASE_URL/author/$AUTHOR_ID" \
  "$AUTHOR_ID"

check_endpoint_contains "rating-catalog get rating" \
  "$API_BASE_URL/rating/$RATING_ISBN" \
  "$RATING_ISBN"

check_endpoint_contains "genre-analysis get genre" \
  "$API_BASE_URL/genre/$GENRE_ID" \
  "$GENRE_ID"

check_endpoint "book-search query" \
  "$API_BASE_URL/book-search?title=Just"

check_endpoint "book-recommendation query" \
  "$API_BASE_URL/book-recommendation?genre=$GENRE_NAME&rating=4&popularity=100"

check_endpoint "compare-service popular low rated" \
  "$API_BASE_URL/compare-service/popular-low-rated?page=1&page_size=1"

check_endpoint "author-analytics rank" \
  "$API_BASE_URL/author-analytics/rank?sort=total_ratings"

printf 'Local integration tests passed.\n'