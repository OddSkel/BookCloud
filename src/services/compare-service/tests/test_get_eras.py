import pytest

from src.handlers.popularity_handler import UnsupportedFilterError, get_eras
from src.models.popularity import PopularityFilters

from tests.conftest import FakeCatalogCache, book


@pytest.mark.asyncio
async def test_get_eras_empty_book_list():
    """Tests: if book list is empty."""
    cache = FakeCatalogCache()

    response = await get_eras(cache, PopularityFilters(), config=None)

    assert response.classic_threshold == 1950
    assert response.modern_threshold == 2000
    assert response.items[0].era == "classic"
    assert response.items[0].n_books == 0
    assert response.items[1].era == "modern"
    assert response.items[1].n_books == 0


@pytest.mark.asyncio
async def test_get_eras_invalid_thresholds_raise_error():
    """Tests: if classic threshold greater or equal to modern raises error."""
    cache = FakeCatalogCache()

    with pytest.raises(UnsupportedFilterError):
        await get_eras(
            cache,
            PopularityFilters(classic_threshold=2000, modern_threshold=2000),
            config=None,
        )


@pytest.mark.asyncio
async def test_get_eras_ignores_books_without_pub_year():
    """Tests: if books without publication year are ignored."""
    cache = FakeCatalogCache(
        books=[
            book(1, "No year", None),
            book(2, "Classic", 1900),
            book(3, "Modern", 2020),
        ],
        ratings_by_isbn={
            2: (100, 4.0),
            3: (50, 5.0),
        },
    )

    response = await get_eras(cache, PopularityFilters(), config=None)

    classic, modern = response.items

    assert classic.n_books == 1
    assert classic.avg_star_rating == 4.0
    assert modern.n_books == 1
    assert modern.avg_star_rating == 5.0


@pytest.mark.asyncio
async def test_get_eras_counts_unrated_books_without_rating_averages():
    """Tests: if unrated books count but averages stay empty."""
    cache = FakeCatalogCache(
        books=[
            book(1, "Unrated classic", 1900),
            book(2, "Unrated modern", 2020),
        ],
        ratings_by_isbn={},
    )

    response = await get_eras(cache, PopularityFilters(), config=None)

    classic, modern = response.items

    assert classic.n_books == 1
    assert classic.avg_star_rating is None
    assert classic.avg_num_ratings is None
    assert classic.sum_num_ratings is None

    assert modern.n_books == 1
    assert modern.avg_star_rating is None
    assert modern.avg_num_ratings is None
    assert modern.sum_num_ratings is None
