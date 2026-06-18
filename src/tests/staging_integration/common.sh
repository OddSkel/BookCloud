#!/usr/bin/env bash

BASE_URL="${BASE_URL:-http://localhost:8080}"
BASE_URL="${BASE_URL%/}"
API_BASE_URL="${API_BASE_URL:-${BASE_URL}/api}"
API_BASE_URL="${API_BASE_URL%/}"

COMMON_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
DATASET_DIR="${DATASET_DIR:-${COMMON_DIR}/../dataset}"
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

require_file() {
  if [[ ! -f "$1" ]]; then
    printf 'Required test dataset file not found: %s\n' "$1" >&2
    exit 1
  fi
}

print_section() {
  printf '\n== %s ==\n' "$1"
}

csv_value() {
  local file="$1"
  local column="$2"
  local row="${3:-1}"

  python3 - "$file" "$column" "$row" <<'PY'
import csv
import sys

path, column, row = sys.argv[1], sys.argv[2], int(sys.argv[3])

with open(path, newline="", encoding="utf-8") as handle:
    reader = csv.DictReader(handle)
    for index, record in enumerate(reader, start=1):
        if index == row:
            print(record.get(column, ""))
            break
PY
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
