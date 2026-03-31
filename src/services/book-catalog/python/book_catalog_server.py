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
        id=row["id"],
        name=row["name"],
        author=row["author"],
        genre=row["genre"],
        year_published=row["year_published"],
        isbn=row["isbn"],
        summary=row["summary"] or "",
    )


class BookCatalogService(book_catalog_pb2_grpc.BookCatalogGrpcServicer):
    def __init__(self, pool: asyncpg.pool.Pool, service_name: str):
        self.pool = pool
        self.service_name = service_name

    async def health_check(self, request, context):
        return common_pb2.HealthCheckResponse(service=self.service_name, status="ok")

    async def GetBooks(self, request, context):
        rows = await self.pool.fetch(
            "SELECT id, name, author, genre, year_published, isbn, summary FROM books"
        )
        return GetBooksResponse(books=[book_row_to_pb(r) for r in rows])

    async def GetBook(self, request, context):
        row = await self.pool.fetchrow(
            "SELECT id, name, author, genre, year_published, isbn, summary FROM books WHERE id = $1",
            request.book_id,
        )
        if row is None:
            context.set_details("Book not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GetBookResponse()
        return GetBookResponse(book=book_row_to_pb(row))

    async def AddBook(self, request, context):
        book = request.book

        if not book.name or not book.author:
            context.set_details("Name and author are required")
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            return AddBookResponse()

        book_id = str(uuid.uuid4())

        row = await self.pool.fetchrow(
            "INSERT INTO books (id, name, author, genre, year_published, isbn, summary) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id, name, author, genre, year_published, isbn, summary",
            book_id,
            book.name,
            book.author,
            book.genre,
            book.year_published,
            book.isbn,
            book.summary if book.summary else None,
        )

        return AddBookResponse(book=book_row_to_pb(row))

    async def UpdateBook(self, request, context):
        book = request.book

        result = await self.pool.fetchrow(
            "UPDATE books SET name=$1, author=$2, genre=$3, year_published=$4, isbn=$5, summary=$6 WHERE id=$7 RETURNING id, name, author, genre, year_published, isbn, summary",
            book.name,
            book.author,
            book.genre,
            book.year_published,
            book.isbn,
            book.summary if book.summary else None,
            request.book_id,
        )

        if result is None:
            context.set_details("Book not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return UpdateBookResponse()

        return UpdateBookResponse(book=book_row_to_pb(result))

    async def DeleteBook(self, request, context):
        res = await self.pool.execute("DELETE FROM books WHERE id = $1", request.book_id)
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
            CREATE TABLE IF NOT EXISTS books (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                author TEXT NOT NULL,
                genre TEXT NOT NULL,
                year_published INTEGER NOT NULL,
                isbn TEXT NOT NULL,
                summary TEXT
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
