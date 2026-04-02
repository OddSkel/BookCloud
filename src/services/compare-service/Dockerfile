FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

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
