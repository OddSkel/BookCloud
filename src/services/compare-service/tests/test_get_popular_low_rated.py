import pytest

from src.handlers.popularity_handler import get_popular_low_rated
from src.models.popularity import PopularityFilters

from tests.conftest import FakeCatalogCache, book


@pytest.mark.asyncio
async def test_get_popular_low_rated_empty_book_list():
    """Tests: if book list is empty."""
    cache = FakeCatalogCache()

    response = await get_popular_low_rated(cache, PopularityFilters(), config=None)

    assert response.items == []
    assert response.total == 0
    assert response.total_pages == 0
    assert response.page == 1
    assert response.page_size == 50


@pytest.mark.asyncio
async def test_get_popular_low_rated_ignores_books_below_default_min_ratings():
    """Tests: if books below default min ratings are ignored."""
    cache = FakeCatalogCache(
        books=[
            book(1, "Few ratings bad book", 2020),
            book(2, "Enough ratings bad book", 2021),
        ],
        ratings_by_isbn={
            1: (19, 1.0),
            2: (20, 1.0),
        },
    )

    response = await get_popular_low_rated(cache, PopularityFilters(), config=None)

    assert response.total == 1
    assert len(response.items) == 1
    assert response.items[0].books.isbn == 2


@pytest.mark.asyncio
async def test_get_popular_low_rated_orders_popular_bad_rated_first():
    """Tests: if popular low rated books are ranked first."""
    cache = FakeCatalogCache(
        books=[
            book(1, "Popular bad", 2020),
            book(2, "Popular good", 2020),
            book(3, "Less popular bad", 2020),
        ],
        ratings_by_isbn={
            1: (1000, 1.5),
            2: (1000, 4.8),
            3: (50, 1.5),
        },
    )

    response = await get_popular_low_rated(
        cache,
        PopularityFilters(page=1, page_size=2),
        config=None,
    )

    assert [item.books.isbn for item in response.items] == [1, 2]
    assert response.total == 3
    assert response.total_pages == 2
