import asyncio
from pathlib import Path
import sys
from typing import TYPE_CHECKING

import grpc

if TYPE_CHECKING:
    from src.models.popularity import PopularityFilters


GENERATED_DIR = Path(__file__).resolve().parent / "generated"
if str(GENERATED_DIR) not in sys.path:
    sys.path.insert(0, str(GENERATED_DIR))

import book_catalog_pb2
import book_catalog_pb2_grpc
import compare_service_pb2
import compare_service_pb2_grpc
import genre_service_pb2
import genre_service_pb2_grpc
import rating_catalog_pb2
import rating_catalog_pb2_grpc

GRPC_MESSAGE_SIZE_LIMIT = 128 * 1024 * 1024


def add_compare_service_to_server(servicer, server) -> None:
    compare_service_pb2_grpc.add_CompareServiceGrpcServicer_to_server(
        servicer,
        server,
    )


async def fetch_book_page(endpoint: str, page_num: int, page_size: int):
    channel = grpc.aio.insecure_channel(
        _grpc_target(endpoint),
        options=_grpc_channel_options(),
    )
    stub = book_catalog_pb2_grpc.BookCatalogGrpcStub(channel)

    try:
        return await stub.GetBooks(
            book_catalog_pb2.GetBooksRequest(
                page_num=page_num,
                page_size=page_size,
            )
        )
    finally:
        await channel.close()


async def iter_book_pages(
    endpoint: str,
    page_size: int = 5000,
    parallelism: int = 8,
):
    first_response = await fetch_book_page(endpoint, page_num=1, page_size=page_size)
    yield list(first_response.books)

    total_pages = first_response.total_pages
    if total_pages <= 1:
        return

    next_page = 2
    while next_page <= total_pages:
        batch_end = min(next_page + parallelism, total_pages + 1)
        responses = await asyncio.gather(
            *[
                fetch_book_page(endpoint, page_num=page_num, page_size=page_size)
                for page_num in range(next_page, batch_end)
            ]
        )

        for response in responses:
            yield list(response.books)

        next_page = batch_end


async def fetch_rating_page(endpoint: str, page_number: int, page_size: int):
    channel = grpc.aio.insecure_channel(
        _grpc_target(endpoint),
        options=_grpc_channel_options(),
    )
    stub = rating_catalog_pb2_grpc.RatingCatalogGrpcStub(channel)

    try:
        return await stub.GetRatings(
            rating_catalog_pb2.GetRatingsRequest(
                page_number=page_number,
                page_size=page_size,
            )
        )
    finally:
        await channel.close()


async def iter_rating_pages(
    endpoint: str,
    page_size: int = 10000,
    parallelism: int = 8,
):
    page_number = 1

    while True:
        batch_end = page_number + parallelism
        responses = await asyncio.gather(
            *[
                fetch_rating_page(
                    endpoint, page_number=current_page, page_size=page_size
                )
                for current_page in range(page_number, batch_end)
            ]
        )

        stop = False
        for response in responses:
            ratings = list(response.ratings)
            if ratings:
                yield ratings

            if len(ratings) < page_size:
                stop = True
                break

        if stop:
            return

        page_number = batch_end


def compare_filters_from_proto(proto_filters) -> "PopularityFilters":
    from src.models.popularity import PopularityFilters

    if proto_filters is None:
        return PopularityFilters()

    return PopularityFilters(
        author_name=proto_filters.author_name
        if proto_filters.HasField("author_name")
        else None,
        author_id=proto_filters.author_id
        if proto_filters.HasField("author_id")
        else None,
        genre_name=list(proto_filters.genre_name),
        genre_id=list(proto_filters.genre_id),
        book_name=proto_filters.book_name
        if proto_filters.HasField("book_name")
        else None,
        book_isbn=proto_filters.book_isbn
        if proto_filters.HasField("book_isbn")
        else None,
        pub_year_from=proto_filters.pub_year_from
        if proto_filters.HasField("pub_year_from")
        else None,
        pub_year_to=proto_filters.pub_year_to
        if proto_filters.HasField("pub_year_to")
        else None,
        pub_year=proto_filters.pub_year if proto_filters.HasField("pub_year") else None,
        page=proto_filters.page if proto_filters.HasField("page") else 1,
        page_size=proto_filters.page_size
        if proto_filters.HasField("page_size")
        else 50,
        min_num_ratings=proto_filters.min_num_ratings
        if proto_filters.HasField("min_num_ratings")
        else None,
        max_num_ratings=proto_filters.max_num_ratings
        if proto_filters.HasField("max_num_ratings")
        else None,
        min_star_rating=proto_filters.min_star_rating
        if proto_filters.HasField("min_star_rating")
        else None,
        max_star_rating=proto_filters.max_star_rating
        if proto_filters.HasField("max_star_rating")
        else None,
        method=proto_filters.method if proto_filters.HasField("method") else None,
        classic_threshold=proto_filters.classic_threshold
        if proto_filters.HasField("classic_threshold")
        else None,
        modern_threshold=proto_filters.modern_threshold
        if proto_filters.HasField("modern_threshold")
        else None,
    )


def _grpc_target(endpoint: str) -> str:
    return endpoint.removeprefix("http://").removeprefix("https://")


def _grpc_channel_options() -> list[tuple[str, int]]:
    return [
        ("grpc.max_receive_message_length", GRPC_MESSAGE_SIZE_LIMIT),
        ("grpc.max_send_message_length", GRPC_MESSAGE_SIZE_LIMIT),
    ]
