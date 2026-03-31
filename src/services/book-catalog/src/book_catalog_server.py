import os
import uuid
import logging
import asyncio

import grpc
import asyncpg
from dotenv import load_dotenv

from book_catalog_pb2 import (
    Book,
    GetBooksResponse,
    GetBookResponse,
    AddBookResponse,
    UpdateBookResponse,
    DeleteBookResponse,
)
import book_catalog_pb2_grpc
import common_pb2


def book_row_to_pb(row) -> Book:
    return Book(
        isbn=row["isbn"],
        name=row["name"],
        url=row["url"] or "",
        summary_clean=row["summary_clean"] or "",
        pub_year=row["pub_year"] or 0,
    )


class BookCatalogService(book_catalog_pb2_grpc.BookCatalogGrpcServicer):
    def __init__(self, pool: asyncpg.pool.Pool, service_name: str):
        self.pool = pool
        self.service_name = service_name

    async def HealthCheck(self, request, context):
        return common_pb2.HealthCheckResponse(service=self.service_name, status="ok")

    async def GetBooks(self, request, context):
        page_num = request.page_num if request.page_num > 0 else 1
        page_size = request.page_size if request.page_size > 0 else 10

        offset = (page_num - 1) * page_size

        total_items_row = await self.pool.fetchrow("SELECT COUNT(*) as cnt FROM book")
        total_items = total_items_row["cnt"] if total_items_row is not None else 0
        total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0

        rows = await self.pool.fetch(
            "SELECT isbn, name, url, summary_clean, pub_year FROM book ORDER BY isbn LIMIT $1 OFFSET $2",
            page_size,
            offset,
        )

        return GetBooksResponse(
            books=[book_row_to_pb(r) for r in rows],
            page_num=page_num,
            page_size=page_size,
            total_items=total_items,
            total_pages=total_pages,
        )

    async def GetBook(self, request, context):
        row = await self.pool.fetchrow(
            "SELECT isbn, name, url, summary_clean, pub_year FROM book WHERE isbn = $1",
            request.isbn,
        )
        if row is None:
            context.set_details("Book not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GetBookResponse()
        return GetBookResponse(book=book_row_to_pb(row))

    async def AddBook(self, request, context):
        book = request.book

        if not book.name or not book.isbn:
            context.set_details("Name and isbn are required")
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            return AddBookResponse()

        row = await self.pool.fetchrow(
            "INSERT INTO book (isbn, name, url, summary_clean, pub_year) VALUES ($1,$2,$3,$4,$5) RETURNING isbn, name, url, summary_clean, pub_year",
            book.isbn,
            book.name,
            book.url if book.url else None,
            book.summary_clean if book.summary_clean else None,
            book.pub_year,
        )

        return AddBookResponse(book=book_row_to_pb(row))

    async def UpdateBook(self, request, context):
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
            return UpdateBookResponse()

        return UpdateBookResponse(book=book_row_to_pb(result))

    async def DeleteBook(self, request, context):
        res = await self.pool.execute("DELETE FROM book WHERE isbn = $1", request.isbn)
        if res.endswith("0"):
            context.set_details("Book not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return DeleteBookResponse()
        return DeleteBookResponse()


async def create_pool():
    user = os.getenv("POSTGRES_USER", "postgres")
    password = os.getenv("POSTGRES_PASSWORD", "postgres")
    host = os.getenv("POSTGRES_HOST", "localhost")
    port = os.getenv("POSTGRES_PORT", "5432")
    db = os.getenv("POSTGRES_DB", "bookdb")

    dsn = f"postgresql://{user}:{password}@{host}:{port}/{db}"
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

    pool = await create_pool()

    server = grpc.aio.server()
    book_catalog_pb2_grpc.add_BookCatalogGrpcServicer_to_server(
        BookCatalogService(pool=pool, service_name=service_name),
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
