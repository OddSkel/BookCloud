from array import array
from collections import defaultdict
from dataclasses import dataclass
import heapq
import math
from itertools import count

from src.grpc import iter_book_pages, iter_rating_pages
from src.models.popularity import (
    BookPQItem,
    ClassicModernComparisonResponse,
    ClassicModernItem,
    CorrelationResponse,
    PaginatedBookPQResponse,
    PopularityFilters,
    PublishingGrowthItem,
    PublishingGrowthResponse,
    PublishingGrowthSummary,
)


class UnsupportedFilterError(ValueError):
    pass


@dataclass
class CatalogBookRecord:
    isbn: int
    name: str
    pub_year: int | None
    star_rating: float | None
    num_ratings: int | None
    authors: list[str]
    genres: list[str]


@dataclass
class YearAccumulator:
    books_published: int = 0
    rated_books: int = 0
    star_sum: float = 0.0
    num_ratings_sum: int = 0


@dataclass
class EraAccumulator:
    n_books: int = 0
    rated_books: int = 0
    star_sum: float = 0.0
    num_ratings_sum: int = 0


async def get_popular_low_rated(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
    config,
) -> PaginatedBookPQResponse:
    _validate_catalog_dependent_filters(filters)

    ratings_by_isbn = await _load_ratings_by_isbn(rating_catalog_grpc_url, config)
    total, min_popularity, max_popularity = await _scan_ranking_stats(
        book_catalog_grpc_url,
        ratings_by_isbn,
        filters,
        config,
    )

    items = await _collect_ranked_items(
        book_catalog_grpc_url,
        ratings_by_isbn,
        filters,
        config,
        min_popularity,
        max_popularity,
        "popular",
    )

    items.sort(
        key=lambda item: (
            -(item.discrepancy_score or 0.0),
            -(item.num_ratings or 0),
            item.star_rating or 0.0,
        )
    )
    return _build_ranked_response(items, total, filters)


async def get_hidden_gems(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
    config,
) -> PaginatedBookPQResponse:
    _validate_catalog_dependent_filters(filters)

    ratings_by_isbn = await _load_ratings_by_isbn(rating_catalog_grpc_url, config)
    total, min_popularity, max_popularity = await _scan_ranking_stats(
        book_catalog_grpc_url,
        ratings_by_isbn,
        filters,
        config,
    )

    items = await _collect_ranked_items(
        book_catalog_grpc_url,
        ratings_by_isbn,
        filters,
        config,
        min_popularity,
        max_popularity,
        "hidden",
    )

    items.sort(
        key=lambda item: (
            item.discrepancy_score or 0.0,
            -(item.star_rating or 0.0),
            item.num_ratings or 0,
        )
    )
    return _build_ranked_response(items, total, filters)


async def get_correlation(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
    config,
) -> CorrelationResponse:
    _validate_catalog_dependent_filters(filters)

    method = (filters.method or "pearson").casefold()
    if method not in {"pearson", "spearman"}:
        raise UnsupportedFilterError(
            "Unsupported correlation method. Use pearson or spearman."
        )

    ratings_by_isbn = await _load_ratings_by_isbn(rating_catalog_grpc_url, config)

    if method == "pearson":
        sample_size, correlation = await _compute_pearson_correlation(
            book_catalog_grpc_url,
            ratings_by_isbn,
            filters,
            config,
        )
    else:
        sample_size, correlation = await _compute_spearman_correlation(
            book_catalog_grpc_url,
            ratings_by_isbn,
            filters,
            config,
        )

    interpretation = (
        "Insufficient data to compute correlation"
        if sample_size < 2
        else _interpret_correlation(correlation)
    )

    return CorrelationResponse(
        method=method,
        correlation=None if correlation is None else round(correlation, 6),
        interpretation=interpretation,
        sample_size=sample_size,
    )


async def get_publishing_growth(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
    config,
) -> PublishingGrowthResponse:
    _validate_catalog_dependent_filters(filters)

    ratings_by_isbn = await _load_ratings_by_isbn(rating_catalog_grpc_url, config)
    grouped: dict[int, YearAccumulator] = defaultdict(YearAccumulator)

    async for record in _iter_catalog_records(book_catalog_grpc_url, ratings_by_isbn, config):
        if not _matches_publishing_growth_filters(record, filters):
            continue
        if record.pub_year is None:
            continue

        accumulator = grouped[record.pub_year]
        accumulator.books_published += 1
        if record.star_rating is not None:
            accumulator.rated_books += 1
            accumulator.star_sum += record.star_rating
            accumulator.num_ratings_sum += record.num_ratings or 0

    items: list[PublishingGrowthItem] = []
    for pub_year in sorted(grouped):
        accumulator = grouped[pub_year]
        items.append(
            PublishingGrowthItem(
                pub_year=pub_year,
                books_published=accumulator.books_published,
                avg_star_rating=(
                    round(accumulator.star_sum / accumulator.rated_books, 6)
                    if accumulator.rated_books
                    else None
                ),
                sum_num_ratings=(
                    accumulator.num_ratings_sum if accumulator.rated_books else None
                ),
            )
        )

    total_books = sum(item.books_published for item in items)
    num_years = len(items)
    summary = PublishingGrowthSummary(
        num_years=num_years,
        total_books=total_books,
        avg_books_years=round(total_books / num_years, 2) if num_years else 0.0,
    )

    return PublishingGrowthResponse(summary=summary, items=items)


async def get_eras(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
    config,
) -> ClassicModernComparisonResponse:
    _validate_catalog_dependent_filters(filters)

    classic_threshold = filters.classic_threshold or 1950
    modern_threshold = filters.modern_threshold or 2000
    if classic_threshold >= modern_threshold:
        raise UnsupportedFilterError(
            "classic_threshold must be smaller than modern_threshold."
        )

    ratings_by_isbn = await _load_ratings_by_isbn(rating_catalog_grpc_url, config)
    classic = EraAccumulator()
    modern = EraAccumulator()

    async for record in _iter_catalog_records(book_catalog_grpc_url, ratings_by_isbn, config):
        if not _matches_eras_filters(record, filters):
            continue
        if record.pub_year is None:
            continue

        if record.pub_year <= classic_threshold:
            _accumulate_era(classic, record)
        if record.pub_year >= modern_threshold:
            _accumulate_era(modern, record)

    return ClassicModernComparisonResponse(
        classic_threshold=classic_threshold,
        modern_threshold=modern_threshold,
        items=[
            _build_era_item("classic", classic),
            _build_era_item("modern", modern),
        ],
    )


async def _load_ratings_by_isbn(rating_catalog_grpc_url: str, config) -> dict[int, tuple[int, float]]:
    ratings_by_isbn: dict[int, tuple[int, float]] = {}

    async for ratings in iter_rating_pages(
        rating_catalog_grpc_url,
        page_size=max(config.rating_page_size, 1),
        parallelism=max(config.parallel_requests, 1),
    ):
        for rating in ratings:
            ratings_by_isbn[int(rating.book_isbn)] = (
                rating.num_ratings,
                rating.star_rating,
            )

    return ratings_by_isbn


async def _iter_catalog_records(book_catalog_grpc_url: str, ratings_by_isbn: dict[int, tuple[int, float]], config):
    async for books in iter_book_pages(
        book_catalog_grpc_url,
        page_size=max(config.book_page_size, 1),
        parallelism=max(config.parallel_requests, 1),
    ):
        for book in books:
            isbn = int(book.isbn)
            rating = ratings_by_isbn.get(isbn)
            yield CatalogBookRecord(
                isbn=isbn,
                name=book.name,
                pub_year=book.pub_year if book.pub_year != 0 else None,
                star_rating=rating[1] if rating else None,
                num_ratings=rating[0] if rating else None,
                authors=[],
                genres=[],
            )


async def _scan_ranking_stats(
    book_catalog_grpc_url: str,
    ratings_by_isbn: dict[int, tuple[int, float]],
    filters: PopularityFilters,
    config,
) -> tuple[int, int, int]:
    total = 0
    min_popularity: int | None = None
    max_popularity: int | None = None

    async for record in _iter_catalog_records(book_catalog_grpc_url, ratings_by_isbn, config):
        if not _matches_listing_filters(record, filters):
            continue

        popularity = record.num_ratings or 0
        total += 1
        min_popularity = popularity if min_popularity is None else min(min_popularity, popularity)
        max_popularity = popularity if max_popularity is None else max(max_popularity, popularity)

    return total, min_popularity or 0, max_popularity or 0


async def _collect_ranked_items(
    book_catalog_grpc_url: str,
    ratings_by_isbn: dict[int, tuple[int, float]],
    filters: PopularityFilters,
    config,
    min_popularity: int,
    max_popularity: int,
    mode: str,
) -> list[BookPQItem]:
    page = max(filters.page, 1)
    page_size = max(filters.page_size, 1)
    keep_limit = page * page_size
    ranked_heap: list[tuple[tuple[float, float, float], int, BookPQItem]] = []
    serial = count()

    async for record in _iter_catalog_records(book_catalog_grpc_url, ratings_by_isbn, config):
        if not _matches_listing_filters(record, filters):
            continue

        item = _record_to_book_item(
            record,
            discrepancy_score=_compute_discrepancy_score(
                record.num_ratings or 0,
                record.star_rating or 0.0,
                min_popularity,
                max_popularity,
            ),
        )
        goodness = (
            _popular_goodness(item) if mode == "popular" else _hidden_goodness(item)
        )
        _push_ranked_item(ranked_heap, keep_limit, goodness, next(serial), item)

    return [entry[2] for entry in ranked_heap]


async def _compute_pearson_correlation(
    book_catalog_grpc_url: str,
    ratings_by_isbn: dict[int, tuple[int, float]],
    filters: PopularityFilters,
    config,
) -> tuple[int, float | None]:
    sample_size = 0
    sum_x = 0.0
    sum_y = 0.0
    sum_xy = 0.0
    sum_x_squared = 0.0
    sum_y_squared = 0.0

    async for record in _iter_catalog_records(book_catalog_grpc_url, ratings_by_isbn, config):
        if not _matches_correlation_filters(record, filters):
            continue

        x = record.star_rating or 0.0
        y = float(record.num_ratings or 0)
        sample_size += 1
        sum_x += x
        sum_y += y
        sum_xy += x * y
        sum_x_squared += x * x
        sum_y_squared += y * y

    if sample_size < 2:
        return sample_size, None

    numerator = sample_size * sum_xy - sum_x * sum_y
    denominator_left = sample_size * sum_x_squared - sum_x * sum_x
    denominator_right = sample_size * sum_y_squared - sum_y * sum_y
    denominator = math.sqrt(max(denominator_left, 0.0) * max(denominator_right, 0.0))

    if denominator == 0:
        return sample_size, 0.0

    return sample_size, numerator / denominator


async def _compute_spearman_correlation(
    book_catalog_grpc_url: str,
    ratings_by_isbn: dict[int, tuple[int, float]],
    filters: PopularityFilters,
    config,
) -> tuple[int, float | None]:
    star_ratings = array("d")
    num_ratings = array("d")

    async for record in _iter_catalog_records(book_catalog_grpc_url, ratings_by_isbn, config):
        if not _matches_correlation_filters(record, filters):
            continue

        star_ratings.append(record.star_rating or 0.0)
        num_ratings.append(float(record.num_ratings or 0))

    sample_size = len(star_ratings)
    if sample_size < 2:
        return sample_size, None

    correlation = _spearman_correlation(star_ratings, num_ratings)
    return sample_size, correlation


# ##############################
# ##############################
# ------------------------------ TODO -------------------------
# ##############################
# ##############################
# Support author_name / author_id once author-catalog exposes lookup RPCs
# and book-catalog exposes book-author relations.
#
# Support genre_name / genre_id once book-catalog exposes book-genre data
# or another catalog service provides genre relations for each book.
def _validate_catalog_dependent_filters(filters: PopularityFilters) -> None:
    unsupported = []
    if filters.author_name:
        unsupported.append("author_name")
    if filters.author_id is not None:
        unsupported.append("author_id")
    if filters.genre_name:
        unsupported.append("genre_name")
    if filters.genre_id:
        unsupported.append("genre_id")

    if unsupported:
        joined = ", ".join(unsupported)
        raise UnsupportedFilterError(
            f"Unsupported filters in current project state: {joined}. "
            "The current upstream contracts do not expose author or genre data."
        )


def _matches_listing_filters(record: CatalogBookRecord, filters: PopularityFilters) -> bool:
    if record.star_rating is None or record.num_ratings is None:
        return False

    return _matches_shared_filters(record, filters)


def _matches_correlation_filters(record: CatalogBookRecord, filters: PopularityFilters) -> bool:
    if record.star_rating is None or record.num_ratings is None:
        return False

    return _matches_shared_filters(record, filters)


def _matches_publishing_growth_filters(
    record: CatalogBookRecord, filters: PopularityFilters
) -> bool:
    return _matches_pub_year_filters(record, filters)


def _matches_eras_filters(record: CatalogBookRecord, filters: PopularityFilters) -> bool:
    return _matches_pub_year_filters(record, filters)


def _matches_shared_filters(record: CatalogBookRecord, filters: PopularityFilters) -> bool:
    if filters.book_name and filters.book_name.casefold() not in record.name.casefold():
        return False

    if filters.book_isbn is not None and record.isbn != filters.book_isbn:
        return False

    if not _matches_pub_year_filters(record, filters):
        return False

    if filters.min_num_ratings is not None and (
        record.num_ratings is None or record.num_ratings < filters.min_num_ratings
    ):
        return False

    if filters.max_num_ratings is not None and (
        record.num_ratings is None or record.num_ratings > filters.max_num_ratings
    ):
        return False

    if filters.min_star_rating is not None and (
        record.star_rating is None or record.star_rating < filters.min_star_rating
    ):
        return False

    if filters.max_star_rating is not None and (
        record.star_rating is None or record.star_rating > filters.max_star_rating
    ):
        return False

    return True


def _matches_pub_year_filters(record: CatalogBookRecord, filters: PopularityFilters) -> bool:
    if filters.pub_year is not None and record.pub_year != filters.pub_year:
        return False

    if filters.pub_year_from is not None and (
        record.pub_year is None or record.pub_year < filters.pub_year_from
    ):
        return False

    if filters.pub_year_to is not None and (
        record.pub_year is None or record.pub_year > filters.pub_year_to
    ):
        return False

    return True


def _record_to_book_item(
    record: CatalogBookRecord,
    discrepancy_score: float | None = None,
) -> BookPQItem:
    return BookPQItem(
        isbn=record.isbn,
        name=record.name,
        pub_year=record.pub_year,
        star_rating=record.star_rating,
        num_ratings=record.num_ratings,
        discrepancy_score=discrepancy_score,
        genres=list(record.genres),
        authors=list(record.authors),
    )


def _build_ranked_response(
    items: list[BookPQItem],
    total: int,
    filters: PopularityFilters,
) -> PaginatedBookPQResponse:
    page = max(filters.page, 1)
    page_size = max(filters.page_size, 1)
    total_pages = math.ceil(total / page_size) if total else 0
    start = (page - 1) * page_size
    end = start + page_size

    return PaginatedBookPQResponse(
        items=items[start:end],
        total=total,
        page=page,
        page_size=page_size,
        total_pages=total_pages,
    )


def _compute_discrepancy_score(
    num_ratings: int,
    star_rating: float,
    min_popularity: int,
    max_popularity: int,
) -> float:
    popularity_norm = _normalize(num_ratings, min_popularity, max_popularity)
    rating_norm = star_rating / 5.0
    return round(popularity_norm - rating_norm, 6)


def _push_ranked_item(
    ranked_heap: list[tuple[tuple[float, float, float], int, BookPQItem]],
    keep_limit: int,
    goodness: tuple[float, float, float],
    serial: int,
    item: BookPQItem,
) -> None:
    if keep_limit <= 0:
        return

    entry = (goodness, serial, item)
    if len(ranked_heap) < keep_limit:
        heapq.heappush(ranked_heap, entry)
        return

    if entry > ranked_heap[0]:
        heapq.heapreplace(ranked_heap, entry)


def _popular_goodness(item: BookPQItem) -> tuple[float, float, float]:
    return (
        item.discrepancy_score or 0.0,
        float(item.num_ratings or 0),
        -(item.star_rating or 0.0),
    )


def _hidden_goodness(item: BookPQItem) -> tuple[float, float, float]:
    return (
        -(item.discrepancy_score or 0.0),
        item.star_rating or 0.0,
        -float(item.num_ratings or 0),
    )


def _normalize(value: int, minimum: int, maximum: int) -> float:
    if maximum == minimum:
        return 1.0 if maximum > 0 else 0.0
    return (value - minimum) / (maximum - minimum)


def _spearman_correlation(xs, ys) -> float | None:
    return _pearson_from_sequences(_rank_values(xs), _rank_values(ys))


def _pearson_from_sequences(xs, ys) -> float | None:
    if len(xs) != len(ys) or len(xs) < 2:
        return None

    mean_x = sum(xs) / len(xs)
    mean_y = sum(ys) / len(ys)
    numerator = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, ys, strict=True))
    sum_sq_x = sum((x - mean_x) ** 2 for x in xs)
    sum_sq_y = sum((y - mean_y) ** 2 for y in ys)
    denominator = math.sqrt(sum_sq_x * sum_sq_y)

    if denominator == 0:
        return 0.0

    return numerator / denominator


def _rank_values(values) -> list[float]:
    ordered = sorted(enumerate(values), key=lambda pair: pair[1])
    ranks = [0.0] * len(values)
    index = 0

    while index < len(ordered):
        start = index
        current_value = ordered[index][1]
        while index < len(ordered) and ordered[index][1] == current_value:
            index += 1

        average_rank = (start + index - 1) / 2 + 1
        for offset in range(start, index):
            original_index = ordered[offset][0]
            ranks[original_index] = average_rank

    return ranks


def _interpret_correlation(correlation: float | None) -> str:
    if correlation is None:
        return "Insufficient data to compute correlation"

    strength = abs(correlation)
    if strength < 0.2:
        intensity = "Very weak"
    elif strength < 0.4:
        intensity = "Weak"
    elif strength < 0.6:
        intensity = "Moderate"
    elif strength < 0.8:
        intensity = "Strong"
    else:
        intensity = "Very strong"

    if correlation > 0:
        direction = "positive"
    elif correlation < 0:
        direction = "negative"
    else:
        return "No correlation"

    return f"{intensity} {direction} correlation"


def _accumulate_era(accumulator: EraAccumulator, record: CatalogBookRecord) -> None:
    accumulator.n_books += 1
    if record.star_rating is not None:
        accumulator.rated_books += 1
        accumulator.star_sum += record.star_rating
        accumulator.num_ratings_sum += record.num_ratings or 0


def _build_era_item(era: str, accumulator: EraAccumulator) -> ClassicModernItem:
    return ClassicModernItem(
        era=era,
        n_books=accumulator.n_books,
        avg_star_rating=(
            round(accumulator.star_sum / accumulator.rated_books, 6)
            if accumulator.rated_books
            else None
        ),
        avg_num_ratings=(
            round(accumulator.num_ratings_sum / accumulator.rated_books, 6)
            if accumulator.rated_books
            else None
        ),
        sum_num_ratings=(
            accumulator.num_ratings_sum if accumulator.rated_books else None
        ),
    )
