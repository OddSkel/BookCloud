import pytest

from src.handlers.popularity_handler import UnsupportedFilterError, get_correlation
from src.models.popularity import PopularityFilters

from tests.conftest import FakeCatalogCache, book


@pytest.mark.asyncio
async def test_get_correlation_unsupported_method_raises_error():
    """Tests: if unsupported correlation method raises an error."""
    cache = FakeCatalogCache()

    with pytest.raises(UnsupportedFilterError):
        await get_correlation(
            cache,
            PopularityFilters(method="kendall"),
            config=None,
        )


@pytest.mark.asyncio
async def test_get_correlation_insufficient_data_returns_none():
    """Tests: if correlation is not computed with less than two records."""
    cache = FakeCatalogCache(
        books=[book(1, "Only rated book", 2020)],
        ratings_by_isbn={1: (10, 4.0)},
    )

    response = await get_correlation(
        cache,
        PopularityFilters(method="pearson"),
        config=None,
    )

    assert response.method == "pearson"
    assert response.sample_size == 1
    assert response.correlation is None
    assert response.interpretation == "Insufficient data to compute correlation"


@pytest.mark.asyncio
async def test_get_correlation_constant_values_returns_zero():
    """Tests: if constant values avoid division by zero."""
    cache = FakeCatalogCache(
        books=[
            book(1, "Book A", 2020),
            book(2, "Book B", 2021),
        ],
        ratings_by_isbn={
            1: (100, 4.0),
            2: (100, 4.0),
        },
    )

    response = await get_correlation(
        cache,
        PopularityFilters(method="pearson"),
        config=None,
    )

    assert response.sample_size == 2
    assert response.correlation == 0.0
    assert response.interpretation == "No correlation"


@pytest.mark.asyncio
async def test_get_correlation_spearman_handles_tied_ranks():
    """Tests: if spearman correlation handles tied values."""
    cache = FakeCatalogCache(
        books=[
            book(1, "Book A", 2020),
            book(2, "Book B", 2021),
            book(3, "Book C", 2022),
        ],
        ratings_by_isbn={
            1: (10, 3.0),
            2: (10, 3.0),
            3: (30, 5.0),
        },
    )

    response = await get_correlation(
        cache,
        PopularityFilters(method="spearman"),
        config=None,
    )

    assert response.method == "spearman"
    assert response.sample_size == 3
    assert response.correlation is not None
