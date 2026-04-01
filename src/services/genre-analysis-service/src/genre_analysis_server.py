import os
import sys
import logging
import asyncio
from datetime import datetime

# Ensure generated_protos directory is on sys.path to support generated files
# that use unqualified imports like `import common_pb2`.
ROOT_DIR = os.path.dirname(__file__)
sys.path.insert(0, ROOT_DIR)
sys.path.insert(0, os.path.join(ROOT_DIR, "generated_protos"))

import grpc
import asyncpg
from dotenv import load_dotenv

from generated_protos.genre_service_pb2 import (
    Genre,
    BookGenre,
    GetGenresResponse,
    GetGenreResponse,
    AddGenreResponse,
    UpdateGenreResponse,
    DeleteGenreResponse,
    AddGenreToBookResponse,
    RemoveGenreFromBookResponse,
    GenreTrendPoint,
    GenreGrowthResponse,
    GenrePopularityResponse,
)
import generated_protos.genre_service_pb2_grpc
import generated_protos.common_pb2


def row_to_genre_pb(row) -> Genre:
    return Genre(
        genre_id=row["genre_id"],
        name=row["name"],
    )


class GenreAnalysisService(generated_protos.genre_service_pb2_grpc.GenreAnalysisGrpcServicer):
    def __init__(self, pool: asyncpg.pool.Pool, service_name: str):
        self.pool = pool
        self.service_name = service_name

    async def HealthCheck(self, request, context):
        return generated_protos.common_pb2.HealthCheckResponse(service=self.service_name, status="ok")

    async def GetGenres(self, request, context):
        page_num = request.page_num if request.page_num > 0 else 1
        page_size = request.page_size if request.page_size > 0 else 10
        offset = (page_num - 1) * page_size

        # existing proto has GenreSort enum with RATING=0 and POPULARITY=1
        # RATING is not available in schema: fallback to name
        sort_order = "ASC" if request.ascending else "DESC"

        if request.sort_by == 1:  # POPULARITY
            total_items_row = await self.pool.fetchrow(
                "SELECT COUNT(*)::int AS cnt FROM genre"
            )
            total_items = total_items_row["cnt"] if total_items_row else 0
            total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0

            rows = await self.pool.fetch(
                "SELECT g.genre_id, g.name, COALESCE(COUNT(bg.book_isbn),0) AS book_count "
                "FROM genre g LEFT JOIN book_genre bg ON g.genre_id = bg.genre_id "
                "GROUP BY g.genre_id, g.name "
                f"ORDER BY book_count {sort_order} LIMIT $1 OFFSET $2",
                page_size,
                offset,
            )
        else:
            total_items_row = await self.pool.fetchrow(
                "SELECT COUNT(*)::int AS cnt FROM genre"
            )
            total_items = total_items_row["cnt"] if total_items_row else 0
            total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0

            rows = await self.pool.fetch(
                f"SELECT genre_id, name FROM genre ORDER BY name {sort_order} LIMIT $1 OFFSET $2",
                page_size,
                offset,
            )

        return GetGenresResponse(
            genres=[row_to_genre_pb(r) for r in rows],
            page_num=page_num,
            page_size=page_size,
            total_items=total_items,
            total_pages=total_pages,
        )

    async def AddGenre(self, request, context):
        genre_in = request.genre
        if not genre_in.name:
            context.set_details("Genre name is required")
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            return AddGenreResponse()

        row = await self.pool.fetchrow(
            "INSERT INTO genre (name) VALUES ($1) RETURNING genre_id,name",
            genre_in.name,
        )

        return AddGenreResponse(genre=row_to_genre_pb(row))

    async def GetGenre(self, request, context):
        row = await self.pool.fetchrow(
            "SELECT genre_id,name FROM genre WHERE genre_id=$1",
            request.genre_id,
        )
        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GetGenreResponse()

        return GetGenreResponse(genre=row_to_genre_pb(row))

    async def UpdateGenre(self, request, context):
        genre_in = request.genre
        row = await self.pool.fetchrow(
            "UPDATE genre SET name=$1 WHERE genre_id=$2 RETURNING genre_id,name",
            genre_in.name,
            request.genre_id,
        )

        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return UpdateGenreResponse()

        return UpdateGenreResponse(genre=row_to_genre_pb(row))

    async def DeleteGenre(self, request, context):
        result = await self.pool.execute("DELETE FROM genre WHERE genre_id=$1", request.genre_id)
        if result.endswith("0"):
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return DeleteGenreResponse()
        return DeleteGenreResponse()

    async def GetGenreGrowth(self, request, context):
        # Using book count in book_genre for basic genre health (no time history available in this schema)
        row = await self.pool.fetchrow(
            "SELECT COUNT(*)::int AS book_count FROM book_genre WHERE genre_id=$1",
            request.genre_id,
        )
        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GenreGrowthResponse()

        points = [GenreTrendPoint(timestamp="", rating=0.0, popularity=row["book_count"])]
        return GenreGrowthResponse(points=points, growth_rate=float(row["book_count"]), metric="book_count")

    async def GetGenrePopularity(self, request, context):
        row = await self.pool.fetchrow(
            "SELECT COUNT(*)::int AS book_count FROM book_genre WHERE genre_id=$1",
            request.genre_id,
        )
        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GenrePopularityResponse()

        points = [GenreTrendPoint(timestamp="", rating=0.0, popularity=row["book_count"])]
        return GenrePopularityResponse(points=points, trend=float(row["book_count"]), metric="book_count")


async def create_pool():
    user = os.getenv("POSTGRES_USER", "postgres")
    password = os.getenv("POSTGRES_PASSWORD", "postgres")
    host = os.getenv("POSTGRES_HOST", "localhost")
    port = os.getenv("POSTGRES_PORT", "5432")
    db = os.getenv("POSTGRES_DB", "db")

    dsn = f"postgresql://{user}:{password}@{host}:{port}/{db}"
    pool = await asyncpg.create_pool(dsn, min_size=1, max_size=5)

    async with pool.acquire() as connection:
        await connection.execute(
            """
            CREATE TABLE IF NOT EXISTS genre (
                genre_id SERIAL PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            )
            """
        )
        await connection.execute(
            """
            CREATE TABLE IF NOT EXISTS book_genre (
                book_isbn BIGINT NOT NULL,
                genre_id INTEGER NOT NULL REFERENCES genre(genre_id) ON DELETE CASCADE,
                PRIMARY KEY (book_isbn, genre_id)
            )
            """
        )

    return pool


async def serve():
    load_dotenv(dotenv_path=os.path.join(os.path.dirname(__file__), "..", ".env"))

    grpc_host = os.getenv("GRPC_HOST", "0.0.0.0")
    grpc_port = int(os.getenv("GRPC_PORT", "50051"))
    service_name = os.getenv("SERVICE_NAME", "genre-analysis")

    pool = await create_pool()

    server = grpc.aio.server()
    generated_protos.genre_service_pb2_grpc.add_GenreAnalysisGrpcServicer_to_server(
        GenreAnalysisService(pool=pool, service_name=service_name),
        server,
    )

    server_addr = f"{grpc_host}:{grpc_port}"
    server.add_insecure_port(server_addr)

    logging.info("Starting GenreAnalysis Python gRPC service on %s", server_addr)
    await server.start()
    await server.wait_for_termination()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(serve())
