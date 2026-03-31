from pathlib import Path
import sys

import grpc


GENERATED_DIR = Path(__file__).resolve().parent / "generated"
if str(GENERATED_DIR) not in sys.path:
    sys.path.insert(0, str(GENERATED_DIR))

import book_catalog_pb2
import book_catalog_pb2_grpc
import compare_service_pb2
import compare_service_pb2_grpc
import rating_catalog_pb2
import rating_catalog_pb2_grpc


def add_compare_service_to_server(servicer, server) -> None:
    compare_service_pb2_grpc.add_CompareServiceGrpcServicer_to_server(
        servicer,
        server,
    )


async def fetch_all_books(endpoint: str, page_size: int = 500):
    channel = grpc.aio.insecure_channel(_grpc_target(endpoint))
    stub = book_catalog_pb2_grpc.BookCatalogGrpcStub(channel)

    try:
        books = []
        page_num = 1
        total_pages = 1

        while page_num <= total_pages:
            response = await stub.GetBooks(
                book_catalog_pb2.GetBooksRequest(
                    page_num=page_num,
                    page_size=page_size,
                )
            )
            books.extend(response.books)

            if response.total_pages <= 0:
                break

            total_pages = response.total_pages
            page_num += 1

        return books
    finally:
        await channel.close()


async def fetch_ratings(endpoint: str):
    channel = grpc.aio.insecure_channel(_grpc_target(endpoint))
    stub = rating_catalog_pb2_grpc.RatingCatalogGrpcStub(channel)

    try:
        response = await stub.GetRatings(rating_catalog_pb2.GetRatingsRequest())
        return list(response.ratings)
    finally:
        await channel.close()


def compare_filters_from_proto(proto_filters) -> "PopularityFilters":
    from src.models.popularity import PopularityFilters

    if proto_filters is None:
        return PopularityFilters()

    return PopularityFilters(
        author_name=proto_filters.author_name if proto_filters.HasField("author_name") else None,
        author_id=proto_filters.author_id if proto_filters.HasField("author_id") else None,
        genre_name=list(proto_filters.genre_name),
        genre_id=list(proto_filters.genre_id),
        book_name=proto_filters.book_name if proto_filters.HasField("book_name") else None,
        book_isbn=proto_filters.book_isbn if proto_filters.HasField("book_isbn") else None,
        pub_year_from=proto_filters.pub_year_from if proto_filters.HasField("pub_year_from") else None,
        pub_year_to=proto_filters.pub_year_to if proto_filters.HasField("pub_year_to") else None,
        pub_year=proto_filters.pub_year if proto_filters.HasField("pub_year") else None,
        page=proto_filters.page if proto_filters.HasField("page") else 1,
        page_size=proto_filters.page_size if proto_filters.HasField("page_size") else 50,
        min_num_ratings=proto_filters.min_num_ratings if proto_filters.HasField("min_num_ratings") else None,
        max_num_ratings=proto_filters.max_num_ratings if proto_filters.HasField("max_num_ratings") else None,
        min_star_rating=proto_filters.min_star_rating if proto_filters.HasField("min_star_rating") else None,
        max_star_rating=proto_filters.max_star_rating if proto_filters.HasField("max_star_rating") else None,
        method=proto_filters.method if proto_filters.HasField("method") else None,
        classic_threshold=proto_filters.classic_threshold if proto_filters.HasField("classic_threshold") else None,
        modern_threshold=proto_filters.modern_threshold if proto_filters.HasField("modern_threshold") else None,
    )


def _grpc_target(endpoint: str) -> str:
    return endpoint.removeprefix("http://").removeprefix("https://")
