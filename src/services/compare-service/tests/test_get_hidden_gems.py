import pytest

from src.handlers.popularity_handler import get_hidden_gems
from src.models.popularity import PopularityFilters

from tests.conftest import FakeCatalogCache, book


@pytest.mark.asyncio
async def test_get_hidden_gems_empty_book_list():
    """Tests: if book list is empty."""
    cache = FakeCatalogCache()

    response = await get_hidden_gems(cache, PopularityFilters(), config=None)

    assert response.items == []
    assert response.total == 0
    assert response.total_pages == 0


@pytest.mark.asyncio
async def test_get_hidden_gems_allows_low_rating_count_when_filter_overrides_default():
    """Tests: if min_num_ratings override allows low popularity books."""
    cache = FakeCatalogCache(
        books=[
            book(1, "Tiny audience excellent book", 2020),
            book(2, "Popular excellent book", 2020),
        ],
        ratings_by_isbn={
            1: (3, 5.0),
            2: (1000, 5.0),
        },
    )

    response = await get_hidden_gems(
        cache,
        PopularityFilters(min_num_ratings=0),
        config=None,
    )

    assert response.total == 2
    assert response.items[0].books.isbn == 1


@pytest.mark.asyncio
async def test_get_hidden_gems_paginates_with_invalid_page_values_safely():
    """Tests: if invalid page values are clamped safely."""
    cache = FakeCatalogCache(
        books=[book(1, "Valid book", 2020)],
        ratings_by_isbn={1: (30, 4.5)},
    )

    response = await get_hidden_gems(
        cache,
        PopularityFilters(page=0, page_size=0),
        config=None,
    )

    assert response.page == 1
    assert response.page_size == 1
    assert response.total_pages == 1
    assert len(response.items) == 1
