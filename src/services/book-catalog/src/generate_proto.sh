#!/bin/sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROTO_ROOT="${PROTO_ROOT:-${SCRIPT_DIR}/../..}"
OUT_DIR="${OUT_DIR:-${SCRIPT_DIR}/generated_protos}"
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