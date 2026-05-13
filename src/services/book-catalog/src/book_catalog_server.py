import os
import sys
import json
import logging
import asyncio
import redis.asyncio as aioredis

ROOT_DIR = os.path.dirname(__file__)
sys.path.insert(0, ROOT_DIR)
sys.path.insert(0, os.path.join(ROOT_DIR, "generated_protos"))

import grpc
import asyncpg
from dotenv import load_dotenv
from prometheus_client import Counter, Histogram, start_http_server

from generated_protos.book_catalog_pb2 import (
    Book,
    GetBooksResponse,
    GetBookResponse,
    AddBookResponse,
    UpdateBookResponse,
    DeleteBookResponse,
)
import generated_protos.book_catalog_pb2_grpc
import generated_protos.common_pb2

SERVICE_NAME = "book-catalog"

BOOKCLOUD_REQUESTS_TOTAL = Counter(
    "bookcloud_requests_total",
    "Total number of requests handled by BookCloud services",
    ["service", "operation", "status"],
)

BOOKCLOUD_REQUEST_DURATION_SECONDS = Histogram(
    "bookcloud_request_duration_seconds",
    "Request duration in seconds for BookCloud services",
    ["service", "operation"],
)


def record_success(operation: str) -> None:
    BOOKCLOUD_REQUESTS_TOTAL.labels(
        service=SERVICE_NAME,
        operation=operation,
        status="success",
    ).inc()


def record_error(operation: str) -> None:
    BOOKCLOUD_REQUESTS_TOTAL.labels(
        service=SERVICE_NAME,
        operation=operation,
        status="error",
    ).inc()


def observe_duration(operation: str):
    return BOOKCLOUD_REQUEST_DURATION_SECONDS.labels(
        service=SERVICE_NAME,
        operation=operation,
    ).time()


def book_row_to_pb(row) -> Book:
    return Book(
        isbn=row["isbn"],
        name=row["name"],
        url=row["url"] or "",
        summary_clean=row["summary_clean"] or "",
        pub_year=row["pub_year"] or 0,
    )


def book_to_dict(b: Book) -> dict:
    return {
        "isbn": b.isbn,
        "name": b.name,
        "url": b.url,
        "summary_clean": b.summary_clean,
        "pub_year": b.pub_year,
    }


class BookCatalogService(generated_protos.book_catalog_pb2_grpc.BookCatalogGrpcServicer):
    def __init__(self, pool: asyncpg.pool.Pool, service_name: str, redis, cache_ttl: int):
        self.pool = pool
        self.service_name = service_name
        self.redis = redis
        self.cache_ttl = cache_ttl

    async def _invalidate_book(self, isbn: int) -> None:
        await self.redis.delete(f"book:{isbn}")
        keys = await self.redis.keys("books:*")
        if keys:
            await self.redis.delete(*keys)

    async def HealthCheck(self, request, context):
        return generated_protos.common_pb2.HealthCheckResponse(
            service=self.service_name,
            status="ok",
        )

    async def GetBooks(self, request, context):
        operation = "list_books"
        with observe_duration(operation):
            try:
                page_num = request.page_num if request.page_num > 0 else 1
                page_size = request.page_size if request.page_size > 0 else 10
                offset = (page_num - 1) * page_size
                author_id = request.author_id if request.HasField("author_id") else None

                cache_key = f"books:page:{page_num}:page_size:{page_size}:author:{author_id}"

                cached = await self.redis.get(cache_key)
                if cached:
                    books_data = json.loads(cached)
                    record_success(operation)
                    return GetBooksResponse(
                        books=[Book(**b) for b in books_data["books"]],
                        page_num=books_data["page_num"],
                        page_size=books_data["page_size"],
                        total_items=books_data["total_items"],
                        total_pages=books_data["total_pages"],
                    )

                if author_id is not None:
                    total_items_row = await self.pool.fetchrow(
                        "SELECT COUNT(*) as cnt FROM book JOIN book_author ON book.isbn = book_author.book_isbn WHERE book_author.author_id = $1",
                        author_id,
                    )
                    total_items = total_items_row["cnt"] if total_items_row is not None else 0
                    total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0

                    rows = await self.pool.fetch(
                        "SELECT book.isbn, book.name, book.url, book.summary_clean, book.pub_year "
                        "FROM book JOIN book_author ON book.isbn = book_author.book_isbn "
                        "WHERE book_author.author_id = $1 "
                        "ORDER BY book.isbn LIMIT $2 OFFSET $3",
                        author_id,
                        page_size,
                        offset,
                    )
                else:
                    total_items_row = await self.pool.fetchrow("SELECT COUNT(*) as cnt FROM book")
                    total_items = total_items_row["cnt"] if total_items_row is not None else 0
                    total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0

                    rows = await self.pool.fetch(
                        "SELECT isbn, name, url, summary_clean, pub_year FROM book ORDER BY isbn LIMIT $1 OFFSET $2",
                        page_size,
                        offset,
                    )

                books = [book_row_to_pb(r) for r in rows]
                response = GetBooksResponse(
                    books=books,
                    page_num=page_num,
                    page_size=page_size,
                    total_items=total_items,
                    total_pages=total_pages,
                )

                serialized = json.dumps({
                    "books": [book_to_dict(b) for b in books],
                    "page_num": page_num,
                    "page_size": page_size,
                    "total_items": total_items,
                    "total_pages": total_pages,
                })
                await self.redis.setex(cache_key, self.cache_ttl, serialized)

                record_success(operation)
                return response

            except Exception:
                record_error(operation)
                raise

    async def GetBook(self, request, context):
        operation = "get_book"
        with observe_duration(operation):
            try:
                cache_key = f"book:{request.isbn}"
                cached = await self.redis.get(cache_key)
                if cached:
                    record_success(operation)
                    return GetBookResponse(book=Book(**json.loads(cached)))

                row = await self.pool.fetchrow(
                    "SELECT isbn, name, url, summary_clean, pub_year FROM book WHERE isbn = $1",
                    request.isbn,
                )
                if row is None:
                    context.set_details("Book not found")
                    context.set_code(grpc.StatusCode.NOT_FOUND)
                    record_success(operation)
                    return GetBookResponse()

                book = book_row_to_pb(row)
                await self.redis.setex(cache_key, self.cache_ttl, json.dumps(book_to_dict(book)))

                record_success(operation)
                return GetBookResponse(book=book)

            except Exception:
                record_error(operation)
                raise

    async def AddBook(self, request, context):
        operation = "create_book"
        with observe_duration(operation):
            try:
                book = request.book

                if not book.name or not book.isbn:
                    context.set_details("Name and isbn are required")
                    context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
                    record_success(operation)
                    return AddBookResponse()

                row = await self.pool.fetchrow(
                    "INSERT INTO book (isbn, name, url, summary_clean, pub_year) VALUES ($1,$2,$3,$4,$5) RETURNING isbn, name, url, summary_clean, pub_year",
                    book.isbn,
                    book.name,
                    book.url if book.url else None,
                    book.summary_clean if book.summary_clean else None,
                    book.pub_year,
                )

                await self._invalidate_book(book.isbn)

                record_success(operation)
                return AddBookResponse(book=book_row_to_pb(row))

            except Exception:
                record_error(operation)
                raise

    async def UpdateBook(self, request, context):
        operation = "update_book"
        with observe_duration(operation):
            try:
                book = request.book

                result = await self.pool.fetchrow(
                    "UPDATE book SET name=$1, url=$2, summary_clean=$3, pub_year=$4 WHERE isbn=$5 RETURNING isbn, name, url, summary_clean, pub_year",
                    book.name,
                    book.url if book.url else None,
                    book.summary_clean if book.summary_clean else None,
                    book.pub_year,
                    request.isbn,
                )

                if result is None:
                    context.set_details("Book not found")
                    context.set_code(grpc.StatusCode.NOT_FOUND)
                    record_success(operation)
                    return UpdateBookResponse()

                await self._invalidate_book(request.isbn)

                record_success(operation)
                return UpdateBookResponse(book=book_row_to_pb(result))

            except Exception:
                record_error(operation)
                raise

    async def DeleteBook(self, request, context):
        operation = "delete_book"
        with observe_duration(operation):
            try:
                res = await self.pool.execute("DELETE FROM book WHERE isbn = $1", request.isbn)
                if res.endswith("0"):
                    context.set_details("Book not found")
                    context.set_code(grpc.StatusCode.NOT_FOUND)
                    record_success(operation)
                    return DeleteBookResponse()

                await self._invalidate_book(request.isbn)

                record_success(operation)
                return DeleteBookResponse()

            except Exception:
                record_error(operation)
                raise


async def create_pool():
    user = os.getenv("POSTGRES_USER", "postgres")
    password = os.getenv("POSTGRES_PASSWORD", "postgres")
    host = os.getenv("POSTGRES_HOST", "localhost")
    db = os.getenv("POSTGRES_DB", "bookdb")

    dsn = f"postgresql://{user}:{password}@{host}:5432/{db}"
    pool = await asyncpg.create_pool(dsn, min_size=1, max_size=5)

    async with pool.acquire() as connection:
        await connection.execute(
            """
            CREATE TABLE IF NOT EXISTS book (
                isbn BIGINT PRIMARY KEY,
                name TEXT NOT NULL,
                url TEXT,
                summary_clean TEXT,
                pub_year INTEGER
            )
            """
        )

    return pool


async def serve():
    load_dotenv(dotenv_path=os.path.join(os.path.dirname(__file__), "..", ".env"))

    grpc_host = os.getenv("GRPC_HOST", "0.0.0.0")
    grpc_port = int(os.getenv("GRPC_PORT", "50051"))
    service_name = os.getenv("SERVICE_NAME", "book-catalog")
    cache_ttl = int(os.getenv("CACHE_TTL", "300"))

    start_http_server(9100)
    logging.info("book-catalog metrics server listening on 0.0.0.0:9100")

    pool = await create_pool()

    redis_url = os.getenv("REDIS_URL", "redis://localhost:6379")
    logging.info("Connecting to Redis at %s", redis_url)
    redis_client = aioredis.from_url(redis_url, decode_responses=True)

    server = grpc.aio.server()
    generated_protos.book_catalog_pb2_grpc.add_BookCatalogGrpcServicer_to_server(
        BookCatalogService(pool=pool, service_name=service_name, redis=redis_client, cache_ttl=cache_ttl),
        server,
    )

    server_addr = f"{grpc_host}:{grpc_port}"
    server.add_insecure_port(server_addr)

    logging.info("Starting BookCatalog Python gRPC service on %s", server_addr)
    await server.start()
    await server.wait_for_termination()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(serve())
