#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "${SCRIPT_DIR}/common.sh"
trap cleanup_response_file EXIT INT TERM

require_command curl
require_command python3

printf 'Running staging acceptance tests against %s\n' "$API_BASE_URL"

discover_test_data

print_section "Acceptance: Reader Finds a Known Book"

request_json "reader searches by title" GET "${API_BASE_URL}/book-search?title=${BOOK_NAME_Q}" 200
json_assert "search includes the known book" "isinstance(data, dict) and isinstance(data.get('books'), list) and any(str(book.get('isbn')) == '${BOOK_ISBN}' for book in data.get('books'))"

request_json "reader opens book details" GET "${API_BASE_URL}/book/${BOOK_ISBN}" 200
json_assert "book detail is usable" "data.get('isbn') == '${BOOK_ISBN}' and data.get('name') == '${BOOK_NAME}' and isinstance(data.get('url'), str) and data.get('url').startswith('http')"

request_json "reader checks rating" GET "${API_BASE_URL}/rating/${RATING_ISBN}" 200
json_assert "rating is visible" "data.get('book_isbn') == '${RATING_ISBN}' and 0 <= float(data.get('star_rating')) <= 5 and int(data.get('num_ratings')) >= 0"

request_json "reader opens author details" GET "${API_BASE_URL}/author/${AUTHOR_ID}" 200
json_assert "author detail is usable" "int(data.get('author_id')) == int('${AUTHOR_ID}') and data.get('name') == '${AUTHOR_NAME}'"

print_section "Acceptance: Reader Discovers Similar Books"

request_json "reader browses genre detail" GET "${API_BASE_URL}/genre/${GENRE_ID}" 200
json_assert "genre detail is usable" "int(data.get('genre_id')) == int('${GENRE_ID}') and data.get('genre_name') == '${GENRE_NAME}' and float(data.get('avg_rating')) >= 0"

request_json "reader asks for recommendations" GET "${API_BASE_URL}/book-recommendation?genre=${GENRE_NAME_Q}&rating=4&popularity=100" 200
json_assert "recommendation response is usable" 'isinstance(data, dict) and isinstance(data.get("books"), list) and all("isbn" in book and "name" in book for book in data.get("books"))'

request_json "reader searches by author" GET "${API_BASE_URL}/book-search?author=${AUTHOR_NAME_Q}" 200
json_assert "author search response is usable" 'isinstance(data, dict) and isinstance(data.get("books"), list)'

print_section "Acceptance: Analyst Explores Catalogue Insights"

request_json "analyst gets popular low-rated books" GET "${API_BASE_URL}/compare-service/popular-low-rated?page=1&page_size=5&min_num_ratings=10" 200
assert_json_object_or_array "popular low-rated response is JSON"

request_json "analyst gets hidden gems" GET "${API_BASE_URL}/compare-service/hidden-gems?page=1&page_size=5&min_star_rating=4" 200
assert_json_object_or_array "hidden gems response is JSON"

request_json "analyst gets author ranking" GET "${API_BASE_URL}/author-analytics/rank?sort=total_ratings" 200
json_assert "author ranking is usable" 'isinstance(data, list) and len(data) > 0 and "author_name" in data[0]'

request_json "analyst gets author performance" GET "${API_BASE_URL}/author-analytics/performance?author_id=${AUTHOR_ID}&pub_year_from=1900&pub_year_to=2026" 200
json_assert "author performance is usable" 'isinstance(data, dict) and "author_name" in data and "sample_size" in data and "evolution" in data'

request_json "analyst gets genre popularity trend" GET "${API_BASE_URL}/genre/${GENRE_ID}/popularity?year_from=2000&year_to=2026" 200
json_assert "genre popularity trend is usable" "int(data.get('genre_id')) == int('${GENRE_ID}') and isinstance(data.get('points'), list) and int(data.get('total_books')) >= 0"

print_section "Acceptance: API Consumer Receives Clear Failures"

request_json "consumer requests invalid book id" GET "${API_BASE_URL}/book/not-an-isbn" 400
json_assert "invalid book response has error" 'isinstance(data, dict) and isinstance(data.get("error"), str) and len(data.get("error")) > 0'

request_json "consumer requests unknown author" GET "${API_BASE_URL}/author/999999999" 404
json_assert "unknown author response has error" 'isinstance(data, dict) and isinstance(data.get("error"), str) and len(data.get("error")) > 0'

printf '\nStaging acceptance tests passed.\n'
