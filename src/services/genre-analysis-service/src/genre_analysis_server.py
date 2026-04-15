import os
import sys
import logging
import asyncio
from collections import defaultdict

ROOT_DIR = os.path.dirname(__file__)
sys.path.insert(0, ROOT_DIR)
sys.path.insert(0, os.path.join(ROOT_DIR, "generated_protos"))

import grpc
import asyncpg
from dotenv import load_dotenv

from generated_protos.genre_service_pb2 import (
    Genre,
    BookGenre,
    GenreWithRating,
    GetGenresResponse,
    AddGenreResponse,
    UpdateGenreResponse,
    DeleteGenreResponse,
    AddGenreToBookResponse,
    RemoveGenreFromBookResponse,
    GenreTrendPoint,
    GenreTrendPointPopularity,
    GenreGrowthResponse,
    GenrePopularityResponse,
)
from generated_protos.genre_service_pb2 import GetGenreResponse
from generated_protos import genre_service_pb2_grpc as genre_service_grpc
from generated_protos import common_pb2
import generated_protos.rating_catalog_pb2 as rating_catalog_pb2
import generated_protos.rating_catalog_pb2_grpc as rating_catalog_grpc
import generated_protos.book_catalog_pb2 as book_catalog_pb2
import generated_protos.book_catalog_pb2_grpc as book_catalog_grpc


def row_to_genre_pb(row) -> Genre:
    return Genre(
        genre_id=row["genre_id"],
        name=row["name"],
    )


class GenreAnalysisService(genre_service_grpc.GenreAnalysisGrpcServicer):
    def __init__(
        self,
        pool: asyncpg.pool.Pool,
        service_name: str,
        rating_catalog_channel: grpc.Channel = None,
        book_catalog_channel: grpc.Channel = None,
    ):
        self.pool = pool
        self.service_name = service_name
        self.rating_catalog_channel = rating_catalog_channel
        self.book_catalog_channel = book_catalog_channel

    async def GetGenres(self, request, context):
        page_num = request.page_num if request.page_num > 0 else 1
        page_size = request.page_size if request.page_size > 0 else 10
        offset = (page_num - 1) * page_size
        sort_by_rating = request.sort_by == 0
        ascending = request.ascending

        genres = await self.pool.fetch("SELECT genre_id, name FROM genre ORDER BY name")
        if not genres:
            return GetGenresResponse(
                genres=[],
                page_num=page_num,
                page_size=page_size,
                total_items=0,
                total_pages=0,
            )

        genre_data = []
        for genre_row in genres:
            genre_id = genre_row["genre_id"]
            genre_name = genre_row["name"]

            book_isbns = await self.pool.fetch(
                "SELECT book_isbn FROM book_genre WHERE genre_id=$1", genre_id
            )

            if not book_isbns:
                continue

            star_sum = 0.0
            num_ratings_sum = 0
            rated_count = 0

            for book_row in book_isbns:
                isbn = str(book_row["book_isbn"])
                try:
                    rating = await self._get_rating_by_isbn(isbn)
                    if rating:
                        star_sum += rating.star_rating
                        num_ratings_sum += rating.num_ratings
                        rated_count += 1
                except Exception as e:
                    logging.warning(f"Failed to get rating for ISBN {isbn}: {e}")
                    continue

            if rated_count == 0:
                continue

            avg_rating = round(star_sum / rated_count, 2)

            genre_data.append(
                {
                    "genre_id": genre_id,
                    "genre_name": genre_name,
                    "avg_rating": avg_rating,
                    "total_num_ratings": num_ratings_sum,
                }
            )

        if sort_by_rating:
            genre_data.sort(key=lambda x: x["avg_rating"], reverse=not ascending)
        else:
            genre_data.sort(key=lambda x: x["total_num_ratings"], reverse=not ascending)

        total_items = len(genre_data)
        total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0
        paginated_data = genre_data[offset : offset + page_size]

        return GetGenresResponse(
            genres=[
                GenreWithRating(
                    rank=offset + idx + 1,
                    genre_id=g["genre_id"],
                    genre_name=g["genre_name"],
                    avg_rating=g["avg_rating"],
                    total_num_ratings=g["total_num_ratings"],
                )
                for idx, g in enumerate(paginated_data)
            ],
            page_num=page_num,
            page_size=page_size,
            total_items=total_items,
            total_pages=total_pages,
        )

    async def _get_rating_by_isbn(self, isbn: str):
        if self.rating_catalog_channel is None:
            return None

        stub = rating_catalog_grpc.RatingCatalogGrpcStub(self.rating_catalog_channel)
        try:
            response = await stub.GetRating(
                rating_catalog_pb2.GetRatingRequest(book_isbn=isbn), timeout=5.0
            )
            return response.rating
        except Exception as e:
            logging.warning(f"Error getting rating for ISBN {isbn}: {e}")
            return None

    async def _get_book_by_isbn(self, isbn: int):
        if self.book_catalog_channel is None:
            return None

        stub = book_catalog_grpc.BookCatalogGrpcStub(self.book_catalog_channel)
        try:
            response = await stub.GetBook(
                book_catalog_pb2.GetBookRequest(isbn=isbn), timeout=5.0
            )
            return response.book
        except Exception as e:
            logging.warning(f"Error getting book for ISBN {isbn}: {e}")
            return None

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
            "SELECT genre_id, name FROM genre WHERE genre_id=$1",
            request.genre_id,
        )
        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GetGenreResponse()

        genre_id = row["genre_id"]
        genre_name = row["name"]

        book_isbns = await self.pool.fetch(
            "SELECT book_isbn FROM book_genre WHERE genre_id=$1", genre_id
        )

        star_sum = 0.0
        num_ratings_sum = 0
        rated_count = 0

        for book_row in book_isbns:
            isbn = str(book_row["book_isbn"])
            try:
                rating = await self._get_rating_by_isbn(isbn)
                if rating:
                    star_sum += rating.star_rating
                    num_ratings_sum += rating.num_ratings
                    rated_count += 1
            except Exception as e:
                logging.warning(f"Failed to get rating for ISBN {isbn}: {e}")
                continue

        avg_rating = round(star_sum / rated_count, 2) if rated_count > 0 else 0.0

        return GetGenreResponse(
            genre_id=genre_id,
            genre_name=genre_name,
            avg_rating=avg_rating,
            total_num_ratings=num_ratings_sum,
        )

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
        result = await self.pool.execute(
            "DELETE FROM genre WHERE genre_id=$1", request.genre_id
        )
        if result.endswith("0"):
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return DeleteGenreResponse()
        return DeleteGenreResponse()

    async def AddGenreToBook(self, request, context):
        book_genre_in = request.bookGenre
        if not book_genre_in.isbn or not book_genre_in.genre_id:
            context.set_details("ISBN and genre_id are required")
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            return AddGenreToBookResponse()

        try:
            await self.pool.execute(
                "INSERT INTO book_genre (book_isbn, genre_id) VALUES ($1, $2) "
                "ON CONFLICT (book_isbn, genre_id) DO NOTHING",
                book_genre_in.isbn,
                book_genre_in.genre_id,
            )
        except Exception as e:
            context.set_details(f"Failed to add genre to book: {e}")
            context.set_code(grpc.StatusCode.INTERNAL)
            return AddGenreToBookResponse()

        return AddGenreToBookResponse(
            bookGenre=BookGenre(
                isbn=book_genre_in.isbn,
                genre_id=book_genre_in.genre_id,
            )
        )

    async def RemoveGenreFromBook(self, request, context):
        book_genre_in = request.bookGenre
        result = await self.pool.execute(
            "DELETE FROM book_genre WHERE book_isbn=$1 AND genre_id=$2",
            book_genre_in.isbn,
            book_genre_in.genre_id,
        )
        if result.endswith("0"):
            context.set_details("Genre-book association not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return RemoveGenreFromBookResponse()
        return RemoveGenreFromBookResponse()

    async def GetGenreGrowth(self, request, context):
        row = await self.pool.fetchrow(
            "SELECT genre_id, name FROM genre WHERE genre_id=$1",
            request.genre_id,
        )
        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GenreGrowthResponse()

        book_isbns = await self.pool.fetch(
            "SELECT book_isbn FROM book_genre WHERE genre_id=$1", request.genre_id
        )

        yearly_data = defaultdict(lambda: {"star_sum": 0.0, "count": 0})
        overall_star_sum = 0.0
        overall_rated_count = 0

        for book_row in book_isbns:
            isbn = book_row["book_isbn"]

            book = await self._get_book_by_isbn(isbn)
            if not book or not book.pub_year:
                continue

            rating = await self._get_rating_by_isbn(str(isbn))
            if not rating:
                continue

            year = book.pub_year
            yearly_data[year]["star_sum"] += rating.star_rating
            yearly_data[year]["count"] += 1

            overall_star_sum += rating.star_rating
            overall_rated_count += 1

        points = [
            GenreTrendPoint(
                year=year,
                avg_rating=round(data["star_sum"] / data["count"], 2)
                if data["count"] > 0
                else 0.0,
            )
            for year, data in sorted(yearly_data.items())
        ]

        overall_avg_rating = (
            round(overall_star_sum / overall_rated_count, 2)
            if overall_rated_count > 0
            else 0.0
        )

        return GenreGrowthResponse(
            points=points,
            avg_rating=overall_avg_rating,
        )

    async def GetGenrePopularity(self, request, context):
        row = await self.pool.fetchrow(
            "SELECT genre_id, name FROM genre WHERE genre_id=$1",
            request.genre_id,
        )
        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GenrePopularityResponse()

        book_isbns = await self.pool.fetch(
            "SELECT book_isbn FROM book_genre WHERE genre_id=$1", request.genre_id
        )

        yearly_data = defaultdict(lambda: {"num_ratings_sum": 0})
        overall_num_ratings_sum = 0

        for book_row in book_isbns:
            isbn = book_row["book_isbn"]

            book = await self._get_book_by_isbn(isbn)
            if not book or not book.pub_year:
                continue

            rating = await self._get_rating_by_isbn(str(isbn))
            if not rating:
                continue

            year = book.pub_year
            yearly_data[year]["num_ratings_sum"] += rating.num_ratings
            overall_num_ratings_sum += rating.num_ratings

        points = [
            GenreTrendPointPopularity(
                year=year,
                total_num_ratings=data["num_ratings_sum"],
            )
            for year, data in sorted(yearly_data.items())
        ]

        return GenrePopularityResponse(
            points=points,
            total_num_ratings=overall_num_ratings_sum,
        )


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
    grpc_port = int(os.getenv("GRPC_PORT", "50055"))
    service_name = os.getenv("SERVICE_NAME", "genre-analysis")
    rating_catalog_host = os.getenv("RATING_CATALOG_HOST", "localhost")
    rating_catalog_port = int(os.getenv("RATING_CATALOG_PORT", "50053"))
    book_catalog_host = os.getenv("BOOK_CATALOG_HOST", "localhost")
    book_catalog_port = int(os.getenv("BOOK_CATALOG_PORT", "50051"))

    pool = await create_pool()

    rating_catalog_channel = grpc.aio.insecure_channel(
        f"{rating_catalog_host}:{rating_catalog_port}"
    )
    book_catalog_channel = grpc.aio.insecure_channel(
        f"{book_catalog_host}:{book_catalog_port}"
    )

    server = grpc.aio.server()
    genre_service_grpc.add_GenreAnalysisGrpcServicer_to_server(
        GenreAnalysisService(
            pool=pool,
            service_name=service_name,
            rating_catalog_channel=rating_catalog_channel,
            book_catalog_channel=book_catalog_channel,
        ),
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
