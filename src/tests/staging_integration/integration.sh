#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "${SCRIPT_DIR}/common.sh"
trap cleanup_response_file EXIT INT TERM

require_command curl
require_command python3

BOOK_CSV="${DATASET_DIR}/book.csv"
AUTHOR_CSV="${DATASET_DIR}/author.csv"
GENRE_CSV="${DATASET_DIR}/genre.csv"
RATING_CSV="${DATASET_DIR}/rating.csv"

require_file "$BOOK_CSV"
require_file "$AUTHOR_CSV"
require_file "$GENRE_CSV"
require_file "$RATING_CSV"

BOOK_ISBN="$(csv_value "$BOOK_CSV" isbn)"
BOOK_NAME="$(csv_value "$BOOK_CSV" name)"
BOOK_YEAR="$(csv_value "$BOOK_CSV" pub_year)"
AUTHOR_ID="$(csv_value "$AUTHOR_CSV" author_id)"
AUTHOR_NAME="$(csv_value "$AUTHOR_CSV" name)"
GENRE_ID="$(csv_value "$GENRE_CSV" genre_id)"
GENRE_NAME="$(csv_value "$GENRE_CSV" name)"
RATING_ISBN="$(csv_value "$RATING_CSV" book_isbn)"

if [[ -z "$BOOK_ISBN" || -z "$BOOK_NAME" || -z "$AUTHOR_ID" || -z "$GENRE_ID" || -z "$RATING_ISBN" ]]; then
  printf 'Test dataset is incomplete. Check files in %s\n' "$DATASET_DIR" >&2
  exit 1
fi

BOOK_NAME_Q="$(urlencode "$BOOK_NAME")"
AUTHOR_NAME_Q="$(urlencode "$AUTHOR_NAME")"
GENRE_NAME_Q="$(urlencode "$GENRE_NAME")"

printf 'Running staging integration tests against %s\n' "$API_BASE_URL"
printf 'Using dataset from %s\n' "$DATASET_DIR"

print_section "Gateway and Catalogs"

request_json "api-gateway health" GET "${API_BASE_URL}/health" 200
json_assert "api-gateway health status" 'data.get("status") == "ok" and data.get("service") == "api-gateway"'

request_json "book-catalog list first page" GET "${API_BASE_URL}/books?page_num=1&page_size=5" 200
json_assert "books paginated list shape" 'isinstance(data, dict) and isinstance(data.get("books"), list) and len(data["books"]) > 0 and len(data["books"]) <= 5 and data.get("page_num") == 1 and data.get("page_size") == 5 and isinstance(data.get("total_items"), int) and isinstance(data.get("total_pages"), int) and all("isbn" in item and "name" in item for item in data["books"])'

request_json "book-catalog known book detail" GET "${API_BASE_URL}/book/${BOOK_ISBN}" 200
json_assert "known book detail" "data.get('isbn') == '${BOOK_ISBN}' and data.get('name') == '${BOOK_NAME}' and int(data.get('pub_year')) == ${BOOK_YEAR}"

request_json "author-catalog list first page" GET "${API_BASE_URL}/authors?page=1&page_size=5" 200
json_assert "authors list shape" 'isinstance(data, list) and len(data) > 0 and all("author_id" in item and "name" in item for item in data)'

request_json "author-catalog known author detail" GET "${API_BASE_URL}/author/${AUTHOR_ID}" 200
json_assert "known author detail" "int(data.get('author_id')) == ${AUTHOR_ID} and data.get('name') == '${AUTHOR_NAME}'"

request_json "rating-catalog list first page" GET "${API_BASE_URL}/ratings?page_number=1&page_size=5" 200
json_assert "ratings list shape" 'isinstance(data, list) and len(data) > 0 and all("book_isbn" in item and "star_rating" in item and "num_ratings" in item for item in data)'

request_json "rating-catalog known rating detail" GET "${API_BASE_URL}/rating/${RATING_ISBN}" 200
json_assert "known rating detail" "data.get('book_isbn') == '${RATING_ISBN}' and float(data.get('star_rating')) > 0 and int(data.get('num_ratings')) >= 0"

print_section "Search, Recommendation and Analytics"

request_json "genre-analysis list first page" GET "${API_BASE_URL}/genres?page_num=1&page_size=5" 200
json_assert "genres list shape" 'isinstance(data, list) and len(data) > 0 and all("genre_id" in item and "genre_name" in item for item in data)'

request_json "genre-analysis known genre detail" GET "${API_BASE_URL}/genre/${GENRE_ID}" 200
json_assert "known genre detail" "int(data.get('genre_id')) == ${GENRE_ID} and data.get('genre_name') == '${GENRE_NAME}'"

request_json "genre-analysis growth" GET "${API_BASE_URL}/genre/${GENRE_ID}/growth?year_from=2000&year_to=2026" 200
json_assert "genre growth shape" "int(data.get('genre_id')) == ${GENRE_ID} and data.get('genre') == '${GENRE_NAME}' and isinstance(data.get('points'), list)"

request_json "genre-analysis popularity" GET "${API_BASE_URL}/genre/${GENRE_ID}/popularity?year_from=2000&year_to=2026" 200
json_assert "genre popularity shape" "int(data.get('genre_id')) == ${GENRE_ID} and data.get('genre') == '${GENRE_NAME}' and isinstance(data.get('points'), list)"

request_json "book-search by title" GET "${API_BASE_URL}/book-search?title=${BOOK_NAME_Q}" 200
json_assert "book-search response shape" 'isinstance(data, dict) and isinstance(data.get("books"), list)'

request_json "book-search by author" GET "${API_BASE_URL}/book-search?author=${AUTHOR_NAME_Q}" 200
json_assert "book-search author response shape" 'isinstance(data, dict) and isinstance(data.get("books"), list)'

request_json "book-recommendation by genre and quality" GET "${API_BASE_URL}/book-recommendation?genre=${GENRE_NAME_Q}&rating=4&popularity=100" 200
json_assert "book-recommendation response shape" 'isinstance(data, dict) and isinstance(data.get("books"), list)'

request_json "compare-service popular low rated" GET "${API_BASE_URL}/compare-service/popular-low-rated?page=1&page_size=3&min_num_ratings=10" 200
assert_json_object_or_array "compare popular-low-rated JSON"

request_json "compare-service hidden gems" GET "${API_BASE_URL}/compare-service/hidden-gems?page=1&page_size=3&min_star_rating=4" 200
assert_json_object_or_array "compare hidden-gems JSON"

request_json "compare-service correlation" GET "${API_BASE_URL}/compare-service/correlation?method=pearson" 200
assert_json_object_or_array "compare correlation JSON"

request_json "compare-service publishing growth" GET "${API_BASE_URL}/compare-service/publishing-growth?pub_year_from=2000&pub_year_to=2026" 200
assert_json_object_or_array "compare publishing-growth JSON"

request_json "compare-service eras" GET "${API_BASE_URL}/compare-service/eras?classic_threshold=2000&modern_threshold=2015" 200
assert_json_object_or_array "compare eras JSON"

request_json "author-analytics ranking by total ratings" GET "${API_BASE_URL}/author-analytics/rank?sort=total_ratings" 200
json_assert "author ranking shape" 'isinstance(data, list) and len(data) > 0 and all("author_name" in item and "total_number_ratings" in item for item in data)'

request_json "author-analytics performance" GET "${API_BASE_URL}/author-analytics/performance?author_id=${AUTHOR_ID}&pub_year_from=1900&pub_year_to=2026" 200
json_assert "author performance shape" 'isinstance(data, dict) and "author_name" in data and "evolution" in data and isinstance(data.get("evolution"), list)'

request_json "author-analytics consistency" GET "${API_BASE_URL}/author-analytics/consistency?author_id=${AUTHOR_ID}" 200
json_assert "author consistency shape" 'isinstance(data, list)'

request_json "author-analytics growth" GET "${API_BASE_URL}/author-analytics/growth?author_id=${AUTHOR_ID}" 200
json_assert "author growth shape" 'isinstance(data, list)'

print_section "Error Contracts"

request_json "invalid ISBN is rejected" GET "${API_BASE_URL}/book/not-an-isbn" 400
json_assert "invalid ISBN error body" 'isinstance(data, dict) and "error" in data'

request_json "missing author returns not found" GET "${API_BASE_URL}/author/999999999" 404
json_assert "missing author error body" 'isinstance(data, dict) and "error" in data'

request_json "missing genre returns error" GET "${API_BASE_URL}/genre/999999999" "404 502"
json_assert "missing genre error body" 'isinstance(data, dict) and "error" in data'

printf '\nStaging integration tests passed.\n'
