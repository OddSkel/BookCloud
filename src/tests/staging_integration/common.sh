#!/usr/bin/env bash

BASE_URL="${BASE_URL:-http://localhost:8080}"
BASE_URL="${BASE_URL%/}"
API_BASE_URL="${API_BASE_URL:-${BASE_URL}/api}"
API_BASE_URL="${API_BASE_URL%/}"

COMMON_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
RESPONSE_FILE="${RESPONSE_FILE:-${TMPDIR:-/tmp}/bookcloud-staging-response.$$}"

# Token priority:
# 1. BOOKCLOUD_ACCESS_TOKEN
# 2. KEYCLOAK_ACCESS_TOKEN
# 3. AUTH_TOKEN
# 4. Dummy JWT for local/CI tests
DUMMY_ACCESS_TOKEN="eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJyZWFsbV9hY2Nlc3MiOnsicm9sZXMiOlsidXNlciJdfX0."

BOOKCLOUD_ACCESS_TOKEN="${BOOKCLOUD_ACCESS_TOKEN:-${KEYCLOAK_ACCESS_TOKEN:-${AUTH_TOKEN:-$DUMMY_ACCESS_TOKEN}}}"

# Header priority:
# 1. AUTHORIZATION_HEADER
# 2. Bearer BOOKCLOUD_ACCESS_TOKEN
if [[ -z "${AUTHORIZATION_HEADER:-}" && -n "$BOOKCLOUD_ACCESS_TOKEN" ]]; then
  AUTHORIZATION_HEADER="Bearer ${BOOKCLOUD_ACCESS_TOKEN}"
fi

cleanup_response_file() {
  rm -f "$RESPONSE_FILE"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

print_section() {
  printf '\n== %s ==\n' "$1"
}

urlencode() {
  python3 - "$1" <<'PY'
import sys
from urllib.parse import quote_plus

print(quote_plus(sys.argv[1]))
PY
}

request_json() {
  local name="$1"
  local method="$2"
  local url="$3"
  local expected_statuses="$4"
  local body="${5:-}"
  local status

  printf 'Testing %-46s %s %s\n' "$name" "$method" "$url"

  local curl_args=(
    -sS
    -L
    -X "$method"
    -o "$RESPONSE_FILE"
    -w "%{http_code}"
    -H "Accept: application/json"
  )

  if [[ -n "${AUTHORIZATION_HEADER:-}" ]]; then
    curl_args+=(-H "Authorization: ${AUTHORIZATION_HEADER}")
  fi

  if [[ -n "$body" ]]; then
    curl_args+=(-H "Content-Type: application/json" --data "$body")
  fi

  status="$(curl "${curl_args[@]}" "$url")"

  if [[ " ${expected_statuses} " != *" ${status} "* ]]; then
    printf 'Unexpected status for "%s": got %s, expected one of [%s]\n' "$name" "$status" "$expected_statuses" >&2

    if [[ -z "${AUTHORIZATION_HEADER:-}" && ( "$status" == "401" || "$status" == "403" ) ]]; then
      printf 'This staging endpoint requires auth. Set BOOKCLOUD_ACCESS_TOKEN, KEYCLOAK_ACCESS_TOKEN, AUTH_TOKEN or AUTHORIZATION_HEADER before running these tests.\n' >&2
    fi

    printf 'Response body:\n' >&2
    cat "$RESPONSE_FILE" >&2 || true
    printf '\n' >&2
    exit 1
  fi
}

json_assert() {
  local description="$1"
  local expression="$2"

  python3 - "$RESPONSE_FILE" "$description" "$expression" <<'PY'
import json
import sys

path, description, expression = sys.argv[1], sys.argv[2], sys.argv[3]

try:
    with open(path, encoding="utf-8") as handle:
        data = json.load(handle)
except Exception as exc:
    print(f'Invalid JSON for "{description}": {exc}', file=sys.stderr)
    with open(path, encoding="utf-8", errors="replace") as handle:
        print(handle.read(), file=sys.stderr)
    sys.exit(1)

scope = {
    "__builtins__": {},
    "data": data,
    "len": len,
    "any": any,
    "all": all,
    "isinstance": isinstance,
    "dict": dict,
    "list": list,
    "str": str,
    "int": int,
    "float": float,
    "abs": abs,
}

try:
    ok = bool(eval(expression, scope, {}))
except Exception as exc:
    print(f'JSON assertion raised for "{description}": {exc}', file=sys.stderr)
    print(json.dumps(data, indent=2, ensure_ascii=False), file=sys.stderr)
    sys.exit(1)

if not ok:
    print(f'JSON assertion failed: {description}', file=sys.stderr)
    print(f'Expression: {expression}', file=sys.stderr)
    print(json.dumps(data, indent=2, ensure_ascii=False), file=sys.stderr)
    sys.exit(1)
PY
}

assert_json_object_or_array() {
  json_assert "$1" 'isinstance(data, (dict, list)) and len(data) >= 0'
}

json_extract() {
  local expression="$1"

  python3 - "$RESPONSE_FILE" "$expression" <<'PY'
import json
import sys

path, expression = sys.argv[1], sys.argv[2]

with open(path, encoding="utf-8") as handle:
    data = json.load(handle)

scope = {
    "__builtins__": {},
    "data": data,
    "len": len,
    "isinstance": isinstance,
    "dict": dict,
    "list": list,
    "str": str,
    "int": int,
    "float": float,
}

value = eval(expression, scope, {})
if value is None:
    value = ""
print(value)
PY
}

discover_test_data() {
  print_section "Discover test data from deployed API"

  request_json "discover first book" GET "${API_BASE_URL}/books?page_num=1&page_size=1" 200

  BOOK_ISBN="$(json_extract 'data.get("books", [{}])[0].get("isbn", "")')"
  BOOK_NAME="$(json_extract 'data.get("books", [{}])[0].get("name", "")')"
  BOOK_YEAR="$(json_extract 'data.get("books", [{}])[0].get("pub_year", "")')"

  request_json "discover first author" GET "${API_BASE_URL}/authors?page=1&page_size=1" 200

  AUTHOR_ID="$(json_extract 'data[0].get("author_id", "") if isinstance(data, list) and data else ""')"
  AUTHOR_NAME="$(json_extract 'data[0].get("name", "") if isinstance(data, list) and data else ""')"

  request_json "discover first genre" GET "${API_BASE_URL}/genres?page_num=1&page_size=1" 200

  GENRE_ID="$(json_extract 'data[0].get("genre_id", "") if isinstance(data, list) and data else ""')"
  GENRE_NAME="$(json_extract 'data[0].get("genre_name", "") if isinstance(data, list) and data else ""')"

  request_json "discover first rating" GET "${API_BASE_URL}/ratings?page_number=1&page_size=1" 200

  RATING_ISBN="$(json_extract 'data[0].get("book_isbn", "") if isinstance(data, list) and data else ""')"

  if [[ -z "$BOOK_ISBN" || -z "$BOOK_NAME" || -z "$AUTHOR_ID" || -z "$GENRE_ID" || -z "$RATING_ISBN" ]]; then
    echo "Não foi possível descobrir dados válidos pela API. Confirma se a fase 06 importou dados." >&2
    echo "BOOK_ISBN=${BOOK_ISBN}" >&2
    echo "BOOK_NAME=${BOOK_NAME}" >&2
    echo "AUTHOR_ID=${AUTHOR_ID}" >&2
    echo "GENRE_ID=${GENRE_ID}" >&2
    echo "RATING_ISBN=${RATING_ISBN}" >&2
    exit 1
  fi

  BOOK_NAME_Q="$(urlencode "$BOOK_NAME")"
  AUTHOR_NAME_Q="$(urlencode "$AUTHOR_NAME")"
  GENRE_NAME_Q="$(urlencode "$GENRE_NAME")"

  export BOOK_ISBN
  export BOOK_NAME
  export BOOK_YEAR
  export AUTHOR_ID
  export AUTHOR_NAME
  export GENRE_ID
  export GENRE_NAME
  export RATING_ISBN
  export BOOK_NAME_Q
  export AUTHOR_NAME_Q
  export GENRE_NAME_Q

  echo "Discovered test data:"
  echo "BOOK_ISBN=${BOOK_ISBN}"
  echo "BOOK_NAME=${BOOK_NAME}"
  echo "BOOK_YEAR=${BOOK_YEAR}"
  echo "AUTHOR_ID=${AUTHOR_ID}"
  echo "AUTHOR_NAME=${AUTHOR_NAME}"
  echo "GENRE_ID=${GENRE_ID}"
  echo "GENRE_NAME=${GENRE_NAME}"
  echo "RATING_ISBN=${RATING_ISBN}"
}