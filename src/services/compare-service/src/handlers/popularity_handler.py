from collections import defaultdict
from dataclasses import dataclass, replace
import math

from src.grpc import fetch_all_books, fetch_ratings
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


async def get_popular_low_rated(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
) -> PaginatedBookPQResponse:
    _validate_catalog_dependent_filters(filters)

    records = await _load_catalog_snapshot(book_catalog_grpc_url, rating_catalog_grpc_url)
    items = [
        _record_to_book_item(record)
        for record in records
        if _matches_listing_filters(record, filters) and record.star_rating is not None
    ]

    _apply_discrepancy_scores(items)
    items.sort(
        key=lambda item: (
            -(item.discrepancy_score or 0.0),
            -(item.num_ratings or 0),
            item.star_rating or 0.0,
        )
    )
    return _paginate_book_items(items, filters)


async def get_hidden_gems(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
) -> PaginatedBookPQResponse:
    _validate_catalog_dependent_filters(filters)

    records = await _load_catalog_snapshot(book_catalog_grpc_url, rating_catalog_grpc_url)
    items = [
        _record_to_book_item(record)
        for record in records
        if _matches_listing_filters(record, filters) and record.star_rating is not None
    ]

    _apply_discrepancy_scores(items)
    items.sort(
        key=lambda item: (
            item.discrepancy_score or 0.0,
            -(item.star_rating or 0.0),
            item.num_ratings or 0,
        )
    )
    return _paginate_book_items(items, filters)


async def get_correlation(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
    filters: PopularityFilters,
) -> CorrelationResponse:
    _validate_catalog_dependent_filters(filters)

    method = (filters.method or "pearson").casefold()
    if method not in {"pearson", "spearman"}:
        raise UnsupportedFilterError(
            "Unsupported correlation method. Use pearson or spearman."
        )

    records = await _load_catalog_snapshot(book_catalog_grpc_url, rating_catalog_grpc_url)
    filtered = [
        record
        for record in records
        if _matches_correlation_filters(record, filters) and record.star_rating is not None
    ]

    star_ratings = [record.star_rating for record in filtered if record.star_rating is not None]
    num_ratings = [float(record.num_ratings) for record in filtered if record.num_ratings is not None]
    sample_size = min(len(star_ratings), len(num_ratings))

    if sample_size < 2:
        correlation = None
        interpretation = "Insufficient data to compute correlation"
    else:
        if method == "pearson":
            correlation = _pearson_correlation(star_ratings, num_ratings)
        else:
            correlation = _spearman_correlation(star_ratings, num_ratings)
        interpretation = _interpret_correlation(correlation)

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
) -> PublishingGrowthResponse:
    _validate_catalog_dependent_filters(filters)

    records = await _load_catalog_snapshot(book_catalog_grpc_url, rating_catalog_grpc_url)
    filtered = [
        record for record in records if _matches_publishing_growth_filters(record, filters)
    ]

    grouped: dict[int, list[CatalogBookRecord]] = defaultdict(list)
    for record in filtered:
        if record.pub_year is None:
            continue
        grouped[record.pub_year].append(record)

    items: list[PublishingGrowthItem] = []
    for pub_year in sorted(grouped):
        year_records = grouped[pub_year]
        rated_records = [record for record in year_records if record.star_rating is not None]
        items.append(
            PublishingGrowthItem(
                pub_year=pub_year,
                books_published=len(year_records),
                avg_star_rating=_rounded_average(
                    [record.star_rating for record in rated_records if record.star_rating is not None]
                ),
                sum_num_ratings=_sum_or_none(
                    [record.num_ratings for record in rated_records if record.num_ratings is not None]
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
) -> ClassicModernComparisonResponse:
    _validate_catalog_dependent_filters(filters)

    classic_threshold = filters.classic_threshold or 1950
    modern_threshold = filters.modern_threshold or 2000
    if classic_threshold >= modern_threshold:
        raise UnsupportedFilterError(
            "classic_threshold must be smaller than modern_threshold."
        )

    records = await _load_catalog_snapshot(book_catalog_grpc_url, rating_catalog_grpc_url)
    filtered = [record for record in records if _matches_eras_filters(record, filters)]

    classic_records = [
        record
        for record in filtered
        if record.pub_year is not None and record.pub_year <= classic_threshold
    ]
    modern_records = [
        record
        for record in filtered
        if record.pub_year is not None and record.pub_year >= modern_threshold
    ]

    return ClassicModernComparisonResponse(
        classic_threshold=classic_threshold,
        modern_threshold=modern_threshold,
        items=[
            _build_era_item("classic", classic_records),
            _build_era_item("modern", modern_records),
        ],
    )


async def _load_catalog_snapshot(
    book_catalog_grpc_url: str,
    rating_catalog_grpc_url: str,
) -> list[CatalogBookRecord]:
    books = await fetch_all_books(book_catalog_grpc_url)
    ratings = await fetch_ratings(rating_catalog_grpc_url)
    ratings_by_isbn = {int(rating.book_isbn): rating for rating in ratings}

    return [
        CatalogBookRecord(
            isbn=int(book.isbn),
            name=book.name,
            pub_year=book.pub_year if book.pub_year != 0 else None,
            star_rating=ratings_by_isbn.get(int(book.isbn)).star_rating
            if int(book.isbn) in ratings_by_isbn
            else None,
            num_ratings=ratings_by_isbn.get(int(book.isbn)).num_ratings
            if int(book.isbn) in ratings_by_isbn
            else None,
            authors=[],
            genres=[],
        )
        for book in books
    ]


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


def _record_to_book_item(record: CatalogBookRecord) -> BookPQItem:
    return BookPQItem(
        isbn=record.isbn,
        name=record.name,
        pub_year=record.pub_year,
        star_rating=record.star_rating,
        num_ratings=record.num_ratings,
        discrepancy_score=None,
        genres=list(record.genres),
        authors=list(record.authors),
    )


def _paginate_book_items(
    items: list[BookPQItem], filters: PopularityFilters
) -> PaginatedBookPQResponse:
    total = len(items)
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


def _apply_discrepancy_scores(items: list[BookPQItem]) -> None:
    if not items:
        return

    popularities = [item.num_ratings or 0 for item in items]
    min_pop = min(popularities)
    max_pop = max(popularities)

    updated_items = []
    for item in items:
        popularity_norm = _normalize(item.num_ratings or 0, min_pop, max_pop)
        rating_norm = (item.star_rating or 0.0) / 5.0
        updated_items.append(
            replace(item, discrepancy_score=round(popularity_norm - rating_norm, 6))
        )

    items[:] = updated_items


def _normalize(value: int, minimum: int, maximum: int) -> float:
    if maximum == minimum:
        return 1.0 if maximum > 0 else 0.0
    return (value - minimum) / (maximum - minimum)


def _pearson_correlation(xs: list[float], ys: list[float]) -> float | None:
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


def _spearman_correlation(xs: list[float], ys: list[float]) -> float | None:
    return _pearson_correlation(_rank_values(xs), _rank_values(ys))


def _rank_values(values: list[float]) -> list[float]:
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


def _build_era_item(era: str, records: list[CatalogBookRecord]) -> ClassicModernItem:
    rated_records = [record for record in records if record.star_rating is not None]
    return ClassicModernItem(
        era=era,
        n_books=len(records),
        avg_star_rating=_rounded_average(
            [record.star_rating for record in rated_records if record.star_rating is not None]
        ),
        avg_num_ratings=_rounded_average(
            [float(record.num_ratings) for record in rated_records if record.num_ratings is not None]
        ),
        sum_num_ratings=_sum_or_none(
            [record.num_ratings for record in rated_records if record.num_ratings is not None]
        ),
    )


def _rounded_average(values: list[float]) -> float | None:
    if not values:
        return None
    return round(sum(values) / len(values), 6)


def _sum_or_none(values: list[int]) -> int | None:
    if not values:
        return None
    return sum(values)
