# syntax=docker/dockerfile:1.7

FROM python:3.12-slim

WORKDIR /app

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

COPY requirements.txt .

RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

COPY ./proto ./proto
COPY ./src ./src

RUN python -m grpc_tools.protoc \
    -I ./proto \
    --python_out=./src/generated \
    --grpc_python_out=./src/generated \
    ./proto/common.proto \
    ./proto/book_catalog.proto \
    ./proto/genre_service.proto \
    ./proto/rating_catalog.proto \
    ./proto/compare_service.proto

CMD ["python", "-m", "src.main"]