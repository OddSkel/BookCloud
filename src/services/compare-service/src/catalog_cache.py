import asyncio
import logging
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

LOGGER = logging.getLogger(__name__)

GRPC_TIMEOUT_SECONDS = 60
MAX_RETRIES_PER_PAGE = 2
GRPC_RETRYABLE_CODES = {
    grpc.StatusCode.UNAVAILABLE,
    grpc.StatusCode.DEADLINE_EXCEEDED,
    grpc.StatusCode.RESOURCE_EXHAUSTED,
    grpc.StatusCode.ABORTED,
}

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

    async def _call_with_retry(self, stub_method, request, name: str):
        for attempt in range(MAX_RETRIES_PER_PAGE):
            try:
                return await stub_method(request, timeout=GRPC_TIMEOUT_SECONDS)
            except grpc.RpcError as exc:
                if attempt < MAX_RETRIES_PER_PAGE - 1 and exc.code() in GRPC_RETRYABLE_CODES:
                    LOGGER.warning(
                        "gRPC %s failed (attempt %d/%d): %s",
                        name, attempt + 1, MAX_RETRIES_PER_PAGE, exc.code().name,
                    )
                    await asyncio.sleep(1)
                    continue
                raise

    async def _load_books(self) -> tuple[CatalogBookSeed, ...]:
        start = time.monotonic()
        page_size = max(self._config.book_page_size, 1)
        parallelism = max(self._config.parallel_requests, 1)

        first_response = await self._call_with_retry(
            self._book_stub.GetBooks,
            _book_page_request(1, page_size),
            "GetBooks/page=1",
        )
        books = [self._book_from_proto(book) for book in first_response.books]

        total_pages = first_response.total_pages
        next_page = 2
        while next_page <= total_pages:
            batch_end = min(next_page + parallelism, total_pages + 1)
            responses = await asyncio.gather(
                *[
                    self._call_with_retry(
                        self._book_stub.GetBooks,
                        _book_page_request(page_num, page_size),
                        f"GetBooks/page={page_num}",
                    )
                    for page_num in range(next_page, batch_end)
                ]
            )

            for response in responses:
                books.extend(self._book_from_proto(book) for book in response.books)

            next_page = batch_end

        elapsed = time.monotonic() - start
        LOGGER.info(
            "Loaded %d books in %.2fs (%d pages, %d parallel)",
            len(books),
            elapsed,
            total_pages,
            parallelism,
        )
        return tuple(books)

    async def _load_ratings_by_isbn(self) -> dict[int, tuple[int, float]]:
        start = time.monotonic()
        page_size = max(self._config.rating_page_size, 1)
        parallelism = max(self._config.parallel_requests, 1)
        ratings_by_isbn: dict[int, tuple[int, float]] = {}
        page_number = 1
        pages_fetched = 0

        while True:
            batch_end = page_number + parallelism
            responses = await asyncio.gather(
                *[
                    self._call_with_retry(
                        self._rating_stub.GetRatings,
                        rating_catalog_pb2.GetRatingsRequest(
                            page_number=current_page,
                            page_size=page_size,
                        ),
                        f"GetRatings/page={current_page}",
                    )
                    for current_page in range(page_number, batch_end)
                ]
            )

            stop = False
            for response in responses:
                pages_fetched += 1
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
                elapsed = time.monotonic() - start
                LOGGER.info(
                    "Loaded %d ratings in %.2fs (%d pages, %d parallel)",
                    len(ratings_by_isbn),
                    elapsed,
                    pages_fetched,
                    parallelism,
                )
                return ratings_by_isbn

            page_number = batch_end

    async def _load_genres_by_isbn(self) -> dict[int, tuple[BookGenreSeed, ...]]:
        start = time.monotonic()
        page_size = max(self._config.genre_page_size, 1)
        genres_by_isbn: dict[int, list[BookGenreSeed]] = {}

        first_response = await self._call_with_retry(
            self._genre_stub.GetBookGenres,
            genre_service_pb2.GetBookGenresRequest(page_num=1, page_size=page_size),
            "GetBookGenres/page=1",
        )
        self._accumulate_book_genres(genres_by_isbn, first_response.items)

        total_pages = first_response.total_pages
        parallelism = max(self._config.parallel_requests, 1)
        next_page = 2
        while next_page <= total_pages:
            batch_end = min(next_page + parallelism, total_pages + 1)
            responses = await asyncio.gather(
                *[
                    self._call_with_retry(
                        self._genre_stub.GetBookGenres,
                        genre_service_pb2.GetBookGenresRequest(
                            page_num=page_num,
                            page_size=page_size,
                        ),
                        f"GetBookGenres/page={page_num}",
                    )
                    for page_num in range(next_page, batch_end)
                ]
            )

            for response in responses:
                self._accumulate_book_genres(genres_by_isbn, response.items)

            next_page = batch_end

        elapsed = time.monotonic() - start
        LOGGER.info(
            "Loaded genres for %d books in %.2fs (%d pages, %d parallel)",
            len(genres_by_isbn),
            elapsed,
            total_pages,
            parallelism,
        )
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


def _book_page_request(page_num: int, page_size: int):
    request = book_catalog_pb2.GetBooksRequest(
        page_num=page_num,
        page_size=page_size,
    )
    if hasattr(request, "include_details"):
        request.include_details = False
    return request
