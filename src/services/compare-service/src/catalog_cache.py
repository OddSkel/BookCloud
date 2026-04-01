import asyncio
import time
from dataclasses import dataclass
from typing import Generic, TypeVar

import grpc

from src.grpc import (
    book_catalog_pb2,
    book_catalog_pb2_grpc,
    genre_service_pb2,
    genre_service_pb2_grpc,
    rating_catalog_pb2,
    rating_catalog_pb2_grpc,
    _grpc_channel_options,
    _grpc_target,
)


T = TypeVar("T")


@dataclass(frozen=True)
class CatalogBookSeed:
    isbn: int
    name: str
    pub_year: int | None


@dataclass(frozen=True)
class BookGenreSeed:
    genre_id: int
    genre_name: str


@dataclass
class _CacheEntry(Generic[T]):
    value: T
    expires_at: float


class _AsyncTTLCache(Generic[T]):
    def __init__(self, ttl_seconds: int) -> None:
        self._ttl_seconds = max(ttl_seconds, 0)
        self._entry: _CacheEntry[T] | None = None
        self._lock = asyncio.Lock()

    async def get(self, loader) -> T:
        now = time.monotonic()
        entry = self._entry
        if entry is not None and entry.expires_at > now:
            return entry.value

        async with self._lock:
            now = time.monotonic()
            entry = self._entry
            if entry is not None and entry.expires_at > now:
                return entry.value

            value = await loader()
            self._entry = _CacheEntry(value=value, expires_at=now + self._ttl_seconds)
            return value


class CatalogDataCache:
    def __init__(self, config) -> None:
        self._config = config
        self._book_channel = grpc.aio.insecure_channel(
            _grpc_target(config.book_catalog_grpc_url),
            options=_grpc_channel_options(),
        )
        self._rating_channel = grpc.aio.insecure_channel(
            _grpc_target(config.rating_catalog_grpc_url),
            options=_grpc_channel_options(),
        )
        self._genre_channel = grpc.aio.insecure_channel(
            _grpc_target(config.genre_analysis_grpc_url),
            options=_grpc_channel_options(),
        )
        self._book_stub = book_catalog_pb2_grpc.BookCatalogGrpcStub(self._book_channel)
        self._rating_stub = rating_catalog_pb2_grpc.RatingCatalogGrpcStub(
            self._rating_channel
        )
        self._genre_stub = genre_service_pb2_grpc.GenreAnalysisGrpcStub(
            self._genre_channel
        )
        self._books_cache = _AsyncTTLCache[tuple[CatalogBookSeed, ...]](
            config.cache_ttl_seconds
        )
        self._ratings_cache = _AsyncTTLCache[dict[int, tuple[int, float]]](
            config.cache_ttl_seconds
        )
        self._genres_cache = _AsyncTTLCache[dict[int, tuple[BookGenreSeed, ...]]](
            config.cache_ttl_seconds
        )

    async def close(self) -> None:
        await asyncio.gather(
            self._book_channel.close(),
            self._genre_channel.close(),
            self._rating_channel.close(),
        )

    async def get_books(self) -> tuple[CatalogBookSeed, ...]:
        return await self._books_cache.get(self._load_books)

    async def get_ratings_by_isbn(self) -> dict[int, tuple[int, float]]:
        return await self._ratings_cache.get(self._load_ratings_by_isbn)

    async def get_genres_by_isbn(self) -> dict[int, tuple[BookGenreSeed, ...]]:
        return await self._genres_cache.get(self._load_genres_by_isbn)

    async def _load_books(self) -> tuple[CatalogBookSeed, ...]:
        page_size = max(self._config.book_page_size, 1)
        parallelism = max(self._config.parallel_requests, 1)

        first_response = await self._book_stub.GetBooks(
            book_catalog_pb2.GetBooksRequest(page_num=1, page_size=page_size)
        )
        books = [self._book_from_proto(book) for book in first_response.books]

        total_pages = first_response.total_pages
        next_page = 2
        while next_page <= total_pages:
            batch_end = min(next_page + parallelism, total_pages + 1)
            responses = await asyncio.gather(
                *[
                    self._book_stub.GetBooks(
                        book_catalog_pb2.GetBooksRequest(
                            page_num=page_num,
                            page_size=page_size,
                        )
                    )
                    for page_num in range(next_page, batch_end)
                ]
            )

            for response in responses:
                books.extend(self._book_from_proto(book) for book in response.books)

            next_page = batch_end

        return tuple(books)

    async def _load_ratings_by_isbn(self) -> dict[int, tuple[int, float]]:
        page_size = max(self._config.rating_page_size, 1)
        parallelism = max(self._config.parallel_requests, 1)
        ratings_by_isbn: dict[int, tuple[int, float]] = {}
        page_number = 1

        while True:
            batch_end = page_number + parallelism
            responses = await asyncio.gather(
                *[
                    self._rating_stub.GetRatings(
                        rating_catalog_pb2.GetRatingsRequest(
                            page_number=current_page,
                            page_size=page_size,
                        )
                    )
                    for current_page in range(page_number, batch_end)
                ]
            )

            stop = False
            for response in responses:
                ratings = response.ratings
                for rating in ratings:
                    ratings_by_isbn[int(rating.book_isbn)] = (
                        rating.num_ratings,
                        rating.star_rating,
                    )

                if len(ratings) < page_size:
                    stop = True
                    break

            if stop:
                return ratings_by_isbn

            page_number = batch_end

    async def _load_genres_by_isbn(self) -> dict[int, tuple[BookGenreSeed, ...]]:
        page_size = max(self._config.genre_page_size, 1)
        genres_by_isbn: dict[int, list[BookGenreSeed]] = {}

        first_response = await self._genre_stub.GetBookGenres(
            genre_service_pb2.GetBookGenresRequest(page_num=1, page_size=page_size)
        )
        self._accumulate_book_genres(genres_by_isbn, first_response.items)

        total_pages = first_response.total_pages
        parallelism = max(self._config.parallel_requests, 1)
        next_page = 2
        while next_page <= total_pages:
            batch_end = min(next_page + parallelism, total_pages + 1)
            responses = await asyncio.gather(
                *[
                    self._genre_stub.GetBookGenres(
                        genre_service_pb2.GetBookGenresRequest(
                            page_num=page_num,
                            page_size=page_size,
                        )
                    )
                    for page_num in range(next_page, batch_end)
                ]
            )

            for response in responses:
                self._accumulate_book_genres(genres_by_isbn, response.items)

            next_page = batch_end

        return {isbn: tuple(entries) for isbn, entries in genres_by_isbn.items()}

    @staticmethod
    def _accumulate_book_genres(
        genres_by_isbn: dict[int, list[BookGenreSeed]],
        items,
    ) -> None:
        for item in items:
            genres_by_isbn.setdefault(int(item.isbn), []).append(
                BookGenreSeed(
                    genre_id=int(item.genre_id),
                    genre_name=item.genre_name,
                )
            )

    @staticmethod
    def _book_from_proto(book) -> CatalogBookSeed:
        return CatalogBookSeed(
            isbn=int(book.isbn),
            name=book.name,
            pub_year=book.pub_year if book.pub_year != 0 else None,
        )
