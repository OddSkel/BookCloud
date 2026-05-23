import pytest
from unittest.mock import AsyncMock, MagicMock
from types import SimpleNamespace


class FakeRating:
    def __init__(self, star_rating, num_ratings):
        self.star_rating = star_rating
        self.num_ratings = num_ratings


class FakeBook:
    def __init__(self, isbn, name, pub_year):
        self.isbn = isbn
        self.name = name
        self.pub_year = pub_year


@pytest.fixture
def mock_pool():
    pool = AsyncMock()
    pool.fetch = AsyncMock()
    pool.fetchrow = AsyncMock()
    pool.execute = AsyncMock()
    return pool


@pytest.fixture
def mock_rating_channel():
    stub = AsyncMock()
    stub.GetRating = AsyncMock()
    channel = MagicMock()
    return channel, stub


@pytest.fixture
def mock_book_channel():
    stub = AsyncMock()
    stub.GetBook = AsyncMock()
    channel = MagicMock()
    return channel, stub


@pytest.fixture
def service(mock_pool, mock_rating_channel, mock_book_channel):
    import sys
    import os
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src", "generated_protos"))

    from genre_analysis_server import GenreAnalysisService
    rating_chan, _ = mock_rating_channel
    book_chan, _ = mock_book_channel
    svc = GenreAnalysisService(
        pool=mock_pool,
        service_name="test",
        rating_catalog_channel=rating_chan,
        book_catalog_channel=book_chan,
    )
    svc.cache_ready = True
    return svc