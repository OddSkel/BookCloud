#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "${SCRIPT_DIR}/common.sh"
trap cleanup_response_file EXIT INT TERM

require_command curl
require_command python3

printf 'Running staging integration tests against %s\n' "$API_BASE_URL"

discover_test_data

print_section "Gateway and Catalogs"

request_json "api-gateway health" GET "${API_BASE_URL}/health" 200
json_assert "api-gateway health status" 'data.get("status") == "ok" and data.get("service") == "api-gateway"'

request_json "book-catalog list first page" GET "${API_BASE_URL}/books?page_num=1&page_size=5" 200
json_assert "books paginated list shape" 'isinstance(data, dict) and isinstance(data.get("books"), list) and len(data["books"]) > 0 and len(data["books"]) <= 5 and data.get("page_num") == 1 and data.get("page_size") == 5 and isinstance(data.get("total_items"), int) and isinstance(data.get("total_pages"), int) and all("isbn" in item and "name" in item for item in data["books"])'

request_json "book-catalog known book detail" GET "${API_BASE_URL}/book/${BOOK_ISBN}" 200
if [[ -n "${BOOK_YEAR:-}" && "$BOOK_YEAR" != "null" ]]; then
  json_assert "known book detail" "data.get('isbn') == '${BOOK_ISBN}' and data.get('name') == '${BOOK_NAME}' and int(data.get('pub_year')) == int('${BOOK_YEAR}')"
else
  json_assert "known book detail" "data.get('isbn') == '${BOOK_ISBN}' and data.get('name') == '${BOOK_NAME}'"
fi

request_json "author-catalog list first page" GET "${API_BASE_URL}/authors?page=1&page_size=5" 200
json_assert "authors list shape" 'isinstance(data, list) and len(data) > 0 and all("author_id" in item and "name" in item for item in data)'

request_json "author-catalog known author detail" GET "${API_BASE_URL}/author/${AUTHOR_ID}" 200
json_assert "known author detail" "int(data.get('author_id')) == int('${AUTHOR_ID}') and data.get('name') == '${AUTHOR_NAME}'"

request_json "rating-catalog list first page" GET "${API_BASE_URL}/ratings?page_number=1&page_size=5" 200
json_assert "ratings list shape" 'isinstance(data, list) and len(data) > 0 and all("book_isbn" in item and "star_rating" in item and "num_ratings" in item for item in data)'

request_json "rating-catalog known rating detail" GET "${API_BASE_URL}/rating/${RATING_ISBN}" 200
json_assert "known rating detail" "data.get('book_isbn') == '${RATING_ISBN}' and float(data.get('star_rating')) >= 0 and int(data.get('num_ratings')) >= 0"

print_section "Search, Recommendation and Analytics"

request_json "genre-analysis list first page" GET "${API_BASE_URL}/genres?page_num=1&page_size=5" 200
json_assert "genres list shape" 'isinstance(data, list) and len(data) > 0 and all("genre_id" in item and "genre_name" in item for item in data)'

request_json "genre-analysis known genre detail" GET "${API_BASE_URL}/genre/${GENRE_ID}" 200
json_assert "known genre detail" "int(data.get('genre_id')) == int('${GENRE_ID}') and data.get('genre_name') == '${GENRE_NAME}'"

request_json "genre-analysis growth" GET "${API_BASE_URL}/genre/${GENRE_ID}/growth?year_from=2000&year_to=2026" 200
json_assert "genre growth shape" "int(data.get('genre_id')) == int('${GENRE_ID}') and isinstance(data.get('points'), list)"

request_json "genre-analysis popularity" GET "${API_BASE_URL}/genre/${GENRE_ID}/popularity?year_from=2000&year_to=2026" 200
json_assert "genre popularity shape" "int(data.get('genre_id')) == int('${GENRE_ID}') and isinstance(data.get('points'), list)"

request_json "book-search by title" GET "${API_BASE_URL}/book-search?title=${BOOK_NAME_Q}" 200
json_assert "book-search response shape" 'isinstance(data, dict) and isinstance(data.get("books"), list)'

request_json "book-recommendation by genre and quality" GET "${API_BASE_URL}/book-recommendation?genre=${GENRE_NAME_Q}&rating=4&popularity=100" 200
json_assert "book-recommendation response shape" 'isinstance(data, dict) and isinstance(data.get("books"), list)'

request_json "compare-service popular low rated" GET "${API_BASE_URL}/compare-service/popular-low-rated?page=1&page_size=3&min_num_ratings=10" 200
assert_json_object_or_array "compare popular-low-rated JSON"

request_json "author-analytics ranking by total ratings" GET "${API_BASE_URL}/author-analytics/rank?sort=total_ratings" 200
json_assert "author ranking shape" 'isinstance(data, list) and len(data) > 0 and all("author_name" in item and "total_number_ratings" in item for item in data)'

request_json "author-analytics performance" GET "${API_BASE_URL}/author-analytics/performance?author_id=${AUTHOR_ID}&pub_year_from=1900&pub_year_to=2026" 200
json_assert "author performance shape" 'isinstance(data, dict) and "author_name" in data and "evolution" in data and isinstance(data.get("evolution"), list)'

request_json "author-analytics consistency" GET "${API_BASE_URL}/author-analytics/consistency?author_id=${AUTHOR_ID}" 200
json_assert "author consistency shape" 'isinstance(data, list)'

request_json "author-analytics growth" GET "${API_BASE_URL}/author-analytics/growth?author_id=${AUTHOR_ID}" 200
json_assert "author growth shape" 'isinstance(data, list)'

printf '\nStaging integration tests passed.\n'