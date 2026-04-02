from dataclasses import asdict, dataclass, field
import json
from typing import Optional


def _compact_payload(value):
    if isinstance(value, dict):
        compacted = {
            key: _compact_payload(item)
            for key, item in value.items()
            if item is not None and item != []
        }
        return compacted

    if isinstance(value, list):
        return [_compact_payload(item) for item in value]

    return value


@dataclass
class PopularityFilters:
    author_name: Optional[str] = None
    author_id: Optional[int] = None
    genre_name: list[str] = field(default_factory=list)
    genre_id: list[int] = field(default_factory=list)
    book_name: Optional[str] = None
    book_isbn: Optional[int] = None
    pub_year_from: Optional[int] = None
    pub_year_to: Optional[int] = None
    pub_year: Optional[int] = None
    page: int = 1
    page_size: int = 50
    min_num_ratings: Optional[int] = None
    max_num_ratings: Optional[int] = None
    min_star_rating: Optional[float] = None
    max_star_rating: Optional[float] = None
    method: Optional[str] = None
    classic_threshold: Optional[int] = None
    modern_threshold: Optional[int] = None


@dataclass
class RankedBook:
    isbn: int
    name: str
    star_rating: Optional[float]
    num_ratings: Optional[int]


@dataclass
class BookPQItem:
    books: RankedBook
    discrepancy_score: Optional[float]


@dataclass
class PaginatedBookPQResponse:
    items: list[BookPQItem]
    total: int
    page: int
    page_size: int
    total_pages: int

    def to_json(self) -> str:
        return json.dumps(_compact_payload(asdict(self)))


@dataclass
class CorrelationResponse:
    method: str
    correlation: Optional[float]
    interpretation: str
    sample_size: int

    def to_json(self) -> str:
        return json.dumps(asdict(self))


@dataclass
class PublishingGrowthItem:
    pub_year: int
    books_published: int
    avg_star_rating: Optional[float]
    sum_num_ratings: Optional[int]


@dataclass
class PublishingGrowthSummary:
    num_years: int
    total_books: int
    avg_books_years: float


@dataclass
class PublishingGrowthResponse:
    summary: PublishingGrowthSummary
    items: list[PublishingGrowthItem]

    def to_json(self) -> str:
        return json.dumps(asdict(self))


@dataclass
class ClassicModernItem:
    era: str
    n_books: int
    avg_star_rating: Optional[float]
    avg_num_ratings: Optional[float]
    sum_num_ratings: Optional[int]


@dataclass
class ClassicModernComparisonResponse:
    classic_threshold: int
    modern_threshold: int
    items: list[ClassicModernItem]

    def to_json(self) -> str:
        return json.dumps(asdict(self))
