#!/usr/bin/env bash
# to run bash or:
# py -3 -m grpc_tools.protoc -I ..\proto --python_out=.\generated_protos --grpc_python_out=.\generated_protos ..\proto\common.proto ..\proto\book_catalog.proto 
set -e

# Resolve directory of this script so output locations are deterministic regardless of cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROTO_ROOT="${PROTO_ROOT:-${SCRIPT_DIR}/../..}"  # can be overridden, e.g. PROTO_ROOT=. in Docker
OUT_DIR="${OUT_DIR:-${SCRIPT_DIR}/generated_protos}"  # default output folder in script directory
PY_OUT="${PY_OUT:-$OUT_DIR}"
GRPC_OUT="${GRPC_OUT:-$OUT_DIR}"

mkdir -p "${PY_OUT}" "${GRPC_OUT}"

python -m grpc_tools.protoc \
  -I${PROTO_ROOT}/proto \
  --python_out=${PY_OUT} \
  --grpc_python_out=${GRPC_OUT} \
  ${PROTO_ROOT}/proto/common.proto \
  ${PROTO_ROOT}/proto/book_catalog.proto

echo "Protobuf classes generated in ${PY_OUT}"