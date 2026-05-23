import os
import sys
import logging
import asyncio
from datetime import datetime
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
    BookGenreInfo,
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
    BookGenreInfo,
    GetBooksByGenreResponse,
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

    async def _get_ratings_batch(self, isbns: list, batch_size: int = 200):
        if not isbns or self.rating_catalog_channel is None:
            return {}

        stub = rating_catalog_grpc.RatingCatalogGrpcStub(self.rating_catalog_channel)
        ratings = {}

        async def fetch_one(isbn):
            try:
                response = await stub.GetRating(
                    rating_catalog_pb2.GetRatingRequest(book_isbn=isbn),
                    timeout=30.0
                )
                return isbn, response.rating
            except Exception as e:
                error_str = str(e)
                if "DEADLINE_EXCEEDED" not in error_str:
                    logging.warning(f"Failed to get rating for ISBN {isbn}: {e}")
                return isbn, None

        for i in range(0, len(isbns), batch_size):
            batch = isbns[i:i + batch_size]
            results = await asyncio.gather(*[fetch_one(isbn) for isbn in batch], return_exceptions=True)
            for isbn, rating in results:
                if isinstance(rating, Exception):
                    continue
                if rating:
                    ratings[isbn] = rating

        return ratings

    async def _get_books_batch(self, isbns: list, batch_size: int = 200):
        if not isbns or self.book_catalog_channel is None:
            return {}

        from generated_protos import book_catalog_pb2
        stub = book_catalog_grpc.BookCatalogGrpcStub(self.book_catalog_channel)
        books = {}

        async def fetch_one(isbn):
            try:
                response = await stub.GetBook(
                    book_catalog_pb2.GetBookRequest(book_isbn=isbn),
                    timeout=30.0
                )
                return isbn, response.book
            except:
                return isbn, None

        for i in range(0, len(isbns), batch_size):
            batch = isbns[i:i + batch_size]
            results = await asyncio.gather(*[fetch_one(isbn) for isbn in batch], return_exceptions=True)
            for isbn, book in results:
                if isinstance(book, Exception):
                    continue
                if book:
                    books[isbn] = book

        return books

    async def GetGenres(self, request, context):
        page_num = request.page_num if request.page_num > 0 else 1
        default_page_size = 20
        page_size = min(request.page_size if request.page_size > 0 else 10, default_page_size)
        sort_by_rating = request.sort_by == 0
        ascending = request.ascending

        genres = await self.pool.fetch(
            """
            SELECT g.genre_id, g.name, COALESCE(c.avg_rating, 0) as avg_rating, 
                  COALESCE(c.total_num_ratings, 0) as total_num_ratings
            FROM genre g
            LEFT JOIN genre_stats_cache c ON g.genre_id = c.genre_id
            ORDER BY g.name
            """
        )

        if not genres:
            return GetGenresResponse(
                genres=[],
                page_num=page_num,
                page_size=page_size,
                total_items=0,
                total_pages=0,
            )

        genre_list = [
            {
                "genre_id": g["genre_id"],
                "genre_name": g["name"],
                "avg_rating": g["avg_rating"],
                "total_num_ratings": g["total_num_ratings"]
            }
            for g in genres
        ]

        genre_list.sort(
            key=lambda x: x["avg_rating"] if sort_by_rating else x["total_num_ratings"],
            reverse=not ascending
        )

        total_items = len(genre_list)
        total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0
        offset = (page_num - 1) * page_size
        paginated = genre_list[offset:offset + page_size]

        return GetGenresResponse(
            genres=[
                GenreWithRating(
                    rank=offset + idx + 1,
                    genre_id=g["genre_id"],
                    genre_name=g["genre_name"],
                    avg_rating=g["avg_rating"],
                    total_num_ratings=g["total_num_ratings"],
                )
                for idx, g in enumerate(paginated)
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
                rating_catalog_pb2.GetRatingRequest(book_isbn=isbn), timeout=30.0
            )
            return response.rating
        except Exception as e:
            error_str = str(e)
            if "DEADLINE_EXCEEDED" not in error_str:
                logging.warning(f"Error getting rating for ISBN {isbn}: {e}")
            return None

    async def _get_book_by_isbn(self, isbn: int):
        if self.book_catalog_channel is None:
            return None

        stub = book_catalog_grpc.BookCatalogGrpcStub(self.book_catalog_channel)
        try:
            response = await stub.GetBook(
                book_catalog_pb2.GetBookRequest(isbn=isbn), timeout=30.0
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
            """
            SELECT g.genre_id, g.name, COALESCE(c.avg_rating, 0) as avg_rating,
                COALESCE(c.total_num_ratings, 0) as total_num_ratings
            FROM genre g
            LEFT JOIN genre_stats_cache c ON g.genre_id = c.genre_id
            WHERE g.genre_id=$1
            """,
            request.genre_id,
        )
        if not row:
            context.set_details("Genre not found")
            context.set_code(grpc.StatusCode.NOT_FOUND)
            return GetGenreResponse()

        genre_id = row["genre_id"]
        genre_name = row["name"]
        avg_rating = row["avg_rating"]
        total_num_ratings = row["total_num_ratings"]

        return GetGenreResponse(
            genre_id=genre_id,
            genre_name=genre_name,
            avg_rating=avg_rating,
            total_num_ratings=total_num_ratings,
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

        year_to = request.year_to if request.year_to > 0 else datetime.now().year
        year_from = request.year_from if request.year_from > 0 else (year_to - 5)

        year_rows = await self.pool.fetch(
            "SELECT year, avg_rating, total_num_ratings FROM genre_year_stats_cache "
            "WHERE genre_id=$1 AND year >= $2 AND year <= $3 ORDER BY year",
            request.genre_id, year_from, year_to,
        )
        points = [
            GenreTrendPoint(year=row["year"], avg_rating=row["avg_rating"])
            for row in year_rows
        ]

        overall_num_ratings = sum(row["total_num_ratings"] for row in year_rows)
        overall_avg_rating = 0.0
        if year_rows:
            total = sum(row["avg_rating"] * row["total_num_ratings"] for row in year_rows)
            if overall_num_ratings > 0:
                overall_avg_rating = round(total / overall_num_ratings, 2)

        return GenreGrowthResponse(
            points=points,
            avg_rating=overall_avg_rating,
            total_num_ratings=overall_num_ratings,
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

        year_to = request.year_to if request.year_to > 0 else datetime.now().year
        year_from = request.year_from if request.year_from > 0 else (year_to - 5)

        year_rows = await self.pool.fetch(
            "SELECT year, total_num_ratings, book_count FROM genre_year_stats_cache "
            "WHERE genre_id=$1 AND year >= $2 AND year <= $3 ORDER BY year",
            request.genre_id, year_from, year_to,
        )
        points = [
            GenreTrendPointPopularity(year=row["year"], total_num_ratings=row["total_num_ratings"])
            for row in year_rows
        ]

        overall_num_ratings = sum(row["total_num_ratings"] for row in year_rows)
        overall_books = sum(row["book_count"] for row in year_rows)

        return GenrePopularityResponse(
            points=points,
            total_num_ratings=overall_num_ratings,
            total_books=overall_books,
        )

    async def GetBooksByGenre(self, request, context):
        if not request.genre_name:
            context.set_details("Genre name is required")
            context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
            return GetBooksByGenreResponse()

        page_num = request.page_num if request.page_num > 0 else 1
        page_size = request.page_size if request.page_size > 0 else 1000
        offset = (page_num - 1) * page_size

        total_items_row = await self.pool.fetchrow(
            "SELECT COUNT(*)::int AS cnt FROM book_genre bg JOIN genre g ON g.genre_id = bg.genre_id WHERE g.name ILIKE $1",
            request.genre_name,
        )
        total_items = total_items_row["cnt"] if total_items_row else 0
        total_pages = (total_items + page_size - 1) // page_size if page_size > 0 else 0

        rows = await self.pool.fetch(
            "SELECT bg.book_isbn, bg.genre_id, g.name AS genre_name "
            "FROM book_genre bg "
            "JOIN genre g ON g.genre_id = bg.genre_id "
            "WHERE g.name ILIKE $1 "
            "ORDER BY bg.book_isbn LIMIT $2 OFFSET $3",
            request.genre_name,
            page_size,
            offset,
        )

        return GetBooksByGenreResponse(
            items=[
                BookGenreInfo(
                    isbn=row["book_isbn"],
                    genre_id=row["genre_id"],
                    genre_name=row["genre_name"],
                )
                for row in rows
            ],
            page_num=page_num,
            page_size=page_size,
            total_items=total_items,
            total_pages=total_pages,
        )


async def create_pool():
    user = os.getenv("POSTGRES_USER", "postgres")
    password = os.getenv("POSTGRES_PASSWORD", "postgres")
    host = os.getenv("POSTGRES_HOST", "localhost")
    port = os.getenv("POSTGRES_PORT", "5432")
    db = os.getenv("POSTGRES_DB", "db")

    dsn = f"postgresql://{user}:{password}@{host}:5432/{db}"
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
            DO $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1
                    FROM pg_constraint c
                    JOIN pg_attribute a
                        ON a.attrelid = c.conrelid
                        AND a.attnum = ANY(c.conkey)
                    WHERE c.conrelid = 'genre'::regclass
                        AND c.contype IN ('p', 'u')
                        AND a.attname = 'genre_id'
                    GROUP BY c.oid
                    HAVING COUNT(*) = 1
                ) THEN
                    ALTER TABLE genre ADD CONSTRAINT genre_genre_id_key UNIQUE (genre_id);
                END IF;
            END
            $$;
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
        await connection.execute(
            """
            CREATE TABLE IF NOT EXISTS genre_stats_cache (
                genre_id INT PRIMARY KEY REFERENCES genre(genre_id) ON DELETE CASCADE,
                avg_rating DOUBLE PRECISION DEFAULT 0,
                total_num_ratings BIGINT DEFAULT 0,
                book_count INT DEFAULT 0,
                updated_at TIMESTAMP DEFAULT NOW()
            )
            """
        )
        await connection.execute(
            """
            ALTER TABLE genre_stats_cache
            ADD COLUMN IF NOT EXISTS book_count INT DEFAULT 0
            """
        )
        await connection.execute(
            """
            CREATE TABLE IF NOT EXISTS genre_year_stats_cache (
                genre_id INT REFERENCES genre(genre_id) ON DELETE CASCADE,
                year INT NOT NULL,
                avg_rating DOUBLE PRECISION DEFAULT 0,
                total_num_ratings BIGINT DEFAULT 0,
                book_count INT DEFAULT 0,
                updated_at TIMESTAMP DEFAULT NOW(),
                PRIMARY KEY (genre_id, year)
            )
            """
        )

    return pool

async def update_genre_cache(pool, rating_catalog_channel, book_catalog_channel, cache_ttl_hours=24):
    if not rating_catalog_channel:
        logging.info("No rating catalog channel, skipping cache update")
        return

    stats_exists = await pool.fetchrow(
        "SELECT COUNT(*) as cnt FROM genre_stats_cache"
    )
    year_exists = await pool.fetchrow(
        "SELECT COUNT(*) as cnt FROM genre_year_stats_cache"
    )

    use_stats = stats_exists["cnt"] == 0
    use_year = year_exists["cnt"] == 0

    if stats_exists["cnt"] > 0:
        stats_cache = await pool.fetchrow(
            "SELECT MAX(updated_at) as last_update FROM genre_stats_cache"
        )
        if stats_cache and stats_cache["last_update"]:
            age_hours = (datetime.now() - stats_cache["last_update"]).total_seconds() / 3600
            if age_hours < cache_ttl_hours:
                use_stats = False
                logging.info(f"genre_stats_cache is fresh ({age_hours:.1f}h old, TTL={cache_ttl_hours}h)")
            else:
                logging.info(f"genre_stats_cache is stale ({age_hours:.1f}h old, TTL={cache_ttl_hours}h), will rebuild")
                await pool.execute("DELETE FROM genre_stats_cache")
        else:
            logging.info("genre_stats_cache has no timestamp, will rebuild")
            await pool.execute("DELETE FROM genre_stats_cache")
    else:
        logging.info("genre_stats_cache is empty, will rebuild")

    if year_exists["cnt"] > 0:
        year_cache = await pool.fetchrow(
            "SELECT MAX(updated_at) as last_update FROM genre_year_stats_cache"
        )
        if year_cache and year_cache["last_update"]:
            age_hours = (datetime.now() - year_cache["last_update"]).total_seconds() / 3600
            if age_hours < cache_ttl_hours:
                use_year = False
                logging.info(f"genre_year_stats_cache is fresh ({age_hours:.1f}h old, TTL={cache_ttl_hours}h)")
            else:
                logging.info(f"genre_year_stats_cache is stale ({age_hours:.1f}h old, TTL={cache_ttl_hours}h), will rebuild")
                await pool.execute("DELETE FROM genre_year_stats_cache")
        else:
            logging.info("genre_year_stats_cache has no timestamp, will rebuild")
            await pool.execute("DELETE FROM genre_year_stats_cache")
    else:
        logging.info("genre_year_stats_cache is empty, will rebuild")

    if not use_stats and not use_year:
        logging.info("All caches are fresh, skipping rebuild")
        return

    if not use_stats:
        await pool.execute("DELETE FROM genre_stats_cache")
    if not use_year:
        await pool.execute("DELETE FROM genre_year_stats_cache")

    all_data = await pool.fetch(
        "SELECT bg.book_isbn, bg.genre_id, g.name as genre_name "
        "FROM book_genre bg JOIN genre g ON bg.genre_id = g.genre_id"
    )
    
    if not all_data:
        logging.info("No books in database, skipping cache update")
        return

    genre_isbns = {}
    all_isbns = []
    
    for row in all_data:
        gid = row["genre_id"]
        isbn = str(row["book_isbn"])
        if gid not in genre_isbns:
            genre_isbns[gid] = {"name": row["genre_name"], "isbns": []}
        genre_isbns[gid]["isbns"].append(isbn)
        all_isbns.append(isbn)

    total_books = len(all_isbns)
    total_genres = len(genre_isbns)
    logging.info(f"Found {total_genres} genres with {total_books} total book-genre entries")

    rating_stub = rating_catalog_grpc.RatingCatalogGrpcStub(rating_catalog_channel)
    batch_size = 300
    logging.info(f"Fetching ratings in batches of {batch_size}...")

    unique_isbns = list(set(all_isbns))
    ratings = {}
    
    async def fetch_rating(isbn):
        try:
            resp = await rating_stub.GetRating(
                rating_catalog_pb2.GetRatingRequest(book_isbn=isbn),
                timeout=30.0
            )
            return isbn, resp.rating
        except:
            return isbn, None

    for i in range(0, len(unique_isbns), batch_size):
        batch = unique_isbns[i:i+batch_size]
        tasks = [fetch_rating(isbn) for isbn in batch]
        results = await asyncio.gather(*tasks, return_exceptions=True)
        for isbn, rating in results:
            if rating and not isinstance(rating, Exception):
                ratings[isbn] = rating
        if i % 3000 == 0:
            logging.info(f"Progress: {i}/{len(unique_isbns)} unique books processed...")

    logging.info(f"Got {len(ratings)} ratings")

    if book_catalog_channel:
        from generated_protos import book_catalog_pb2
        book_stub = book_catalog_grpc.BookCatalogGrpcStub(book_catalog_channel)
        logging.info(f"Fetching books (pub_year) in batches of {batch_size}...")

        books = {}
        
        async def fetch_book(isbn):
            try:
                resp = await book_stub.GetBook(
                    book_catalog_pb2.GetBookRequest(isbn=int(isbn)),
                    timeout=30.0
                )
                return isbn, resp.book
            except Exception as e:
                logging.warning(f"Failed to get book {isbn}: {e}")
                return isbn, None

        for i in range(0, len(unique_isbns), batch_size):
            batch = unique_isbns[i:i+batch_size]
            tasks = [fetch_book(isbn) for isbn in batch]
            results = await asyncio.gather(*tasks, return_exceptions=True)
            for isbn, book in results:
                if book and not isinstance(book, Exception):
                    books[isbn] = book
            if i % 3000 == 0:
                logging.info(f"Progress: {i}/{len(unique_isbns)} books processed...")

        logging.info(f"Got {len(books)} books")

    logging.info("Computing genre stats...")

    if not use_stats:
        logging.info("Skipping genre_stats_cache (already fresh)")
    else:
        for gid, data in genre_isbns.items():
            isbns = data["isbns"]
            star_sum = 0.0
            num_ratings_sum = 0
            rated_count = 0

            for isbn in isbns:
                rating = ratings.get(isbn)
                if rating:
                    star_sum += rating.star_rating
                    num_ratings_sum += rating.num_ratings
                    rated_count += 1

            avg_rating = round(star_sum / rated_count, 2) if rated_count > 0 else 0.0

            await pool.execute(
                """
                INSERT INTO genre_stats_cache (genre_id, avg_rating, total_num_ratings, book_count, updated_at)
                VALUES ($1, $2, $3, $4, NOW())
                ON CONFLICT (genre_id) DO UPDATE SET
                    avg_rating = EXCLUDED.avg_rating,
                    total_num_ratings = EXCLUDED.total_num_ratings,
                    book_count = EXCLUDED.book_count,
                    updated_at = NOW()
                """,
                gid, avg_rating, num_ratings_sum, len(isbns)
            )

    if use_year and book_catalog_channel and books:
        logging.info("Computing genre year stats...")
        total = len(unique_isbns)
        count = 0

        for gid, data in genre_isbns.items():
            isbns = data["isbns"]
            yearly = defaultdict(lambda: {"star_sum": 0.0, "num_ratings_sum": 0, "count": 0})

            for isbn in isbns:
                count += 1
                rating = ratings.get(isbn)
                book = books.get(isbn)
                if rating and book and book.pub_year:
                    year = book.pub_year
                    yearly[year]["star_sum"] += rating.star_rating
                    yearly[year]["num_ratings_sum"] += rating.num_ratings
                    yearly[year]["count"] += 1

            for year, ydata in yearly.items():
                avg = round(ydata["star_sum"] / ydata["count"], 2) if ydata["count"] > 0 else 0.0
                await pool.execute(
                    """
                    INSERT INTO genre_year_stats_cache (genre_id, year, avg_rating, total_num_ratings, book_count, updated_at)
                    VALUES ($1, $2, $3, $4, $5, NOW())
                    ON CONFLICT (genre_id, year) DO UPDATE SET
                        avg_rating = EXCLUDED.avg_rating,
                        total_num_ratings = EXCLUDED.total_num_ratings,
                        book_count = EXCLUDED.book_count,
                        updated_at = NOW()
                    """,
                    gid, year, avg, ydata["num_ratings_sum"], ydata["count"]
                )

    logging.info("Genre cache updated successfully")


async def serve():
    load_dotenv(dotenv_path=os.path.join(os.path.dirname(__file__), "..", ".env"))

    grpc_host = os.getenv("GRPC_HOST", "0.0.0.0")
    grpc_port = int(os.getenv("GRPC_PORT", "50055"))
    service_name = os.getenv("SERVICE_NAME", "genre-analysis")
    rating_catalog_host = os.getenv("RATING_CATALOG_HOST", "localhost")
    rating_catalog_port = int(os.getenv("RATING_CATALOG_GRPC_PORT", "50053"))
    book_catalog_port = int(os.getenv("BOOK_CATALOG_GRPC_PORT", "50051"))
    book_catalog_host = os.getenv("BOOK_CATALOG_HOST", "localhost")

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

    async def update_cache_in_background():
        logging.info("Updating genre cache (this may take several minutes)...")
        try:
            await update_genre_cache(pool, rating_catalog_channel, book_catalog_channel)
        except Exception as e:
            logging.warning(f"Failed to update cache: {e}")

    asyncio.create_task(update_cache_in_background())
    await server.wait_for_termination()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(serve())
