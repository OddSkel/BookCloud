import pytest

from src.handlers.popularity_handler import get_publishing_growth
from src.models.popularity import PopularityFilters

from tests.conftest import FakeCatalogCache, book


@pytest.mark.asyncio
async def test_get_publishing_growth_empty_book_list():
    """Tests: if book list is empty."""
    cache = FakeCatalogCache()

    response = await get_publishing_growth(cache, PopularityFilters(), config=None)

    assert response.items == []
    assert response.summary.num_years == 0
    assert response.summary.total_books == 0
    assert response.summary.avg_books_years == 0.0


@pytest.mark.asyncio
async def test_get_publishing_growth_ignores_books_without_pub_year():
    """Tests: if books without publication year are ignored."""
    cache = FakeCatalogCache(
        books=[
            book(1, "No year", None),
            book(2, "Has year", 2020),
        ],
        ratings_by_isbn={
            1: (10, 4.0),
            2: (20, 5.0),
        },
    )

    response = await get_publishing_growth(cache, PopularityFilters(), config=None)

    assert len(response.items) == 1
    assert response.items[0].pub_year == 2020
    assert response.summary.total_books == 1


@pytest.mark.asyncio
async def test_get_publishing_growth_keeps_unrated_books_but_avg_is_none():
    """Tests: if unrated books count but rating aggregates stay empty."""
    cache = FakeCatalogCache(
        books=[book(1, "Unrated book", 1999)],
        ratings_by_isbn={},
    )

    response = await get_publishing_growth(cache, PopularityFilters(), config=None)

    assert response.items[0].books_published == 1
    assert response.items[0].avg_star_rating is None
    assert response.items[0].sum_num_ratings is None
    assert response.summary.avg_books_years == 1.0


@pytest.mark.asyncio
async def test_get_publishing_growth_filters_year_range_boundaries():
    """Tests: if publication year range is inclusive."""
    cache = FakeCatalogCache(
        books=[
            book(1, "Before", 1999),
            book(2, "Start", 2000),
            book(3, "End", 2001),
            book(4, "After", 2002),
        ],
        ratings_by_isbn={
            1: (10, 4.0),
            2: (10, 4.0),
            3: (10, 4.0),
            4: (10, 4.0),
        },
    )

    response = await get_publishing_growth(
        cache,
        PopularityFilters(pub_year_from=2000, pub_year_to=2001),
        config=None,
    )

    assert [item.pub_year for item in response.items] == [2000, 2001]
    assert response.summary.total_books == 2
