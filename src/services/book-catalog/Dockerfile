FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY ./proto ./proto
COPY ./src/book_catalog_server.py .
COPY ./src/generate_proto.sh .

RUN chmod +x generate_proto.sh \
 && PROTO_ROOT="." ./generate_proto.sh

EXPOSE 50051

CMD ["python", "book_catalog_server.py"]
