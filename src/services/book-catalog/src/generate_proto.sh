#!/bin/bash
set -e

PROTO_ROOT="${PROTO_ROOT:-../..}"  # can be overridden, e.g. PROTO_ROOT=. in Docker
PY_OUT="."
GRPC_OUT="."

python -m grpc_tools.protoc \
  -I${PROTO_ROOT}/proto \
  --python_out=${PY_OUT} \
  --grpc_python_out=${GRPC_OUT} \
  ${PROTO_ROOT}/proto/common.proto \
  ${PROTO_ROOT}/proto/book_catalog.proto

echo "Protobuf classes generated in ${PY_OUT}"