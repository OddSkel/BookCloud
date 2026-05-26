#!/usr/bin/env bash

set -Eeuo pipefail

SCRIPT_DIR="$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"

"${SCRIPT_DIR}/integration.sh"
"${SCRIPT_DIR}/acceptance.sh"

printf '\nAll staging integration and acceptance tests passed.\n'
