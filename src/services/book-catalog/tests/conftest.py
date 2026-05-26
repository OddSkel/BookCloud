import pytest
from unittest.mock import AsyncMock, MagicMock
from types import SimpleNamespace


class FakeRedis:
    def __init__(self):
        self._data = {}

    async def get(self, key):
        return self._data.get(key)

    async def setex(self, key, ttl, value):
        self._data[key] = value

    async def delete(self, *keys):
        for k in keys:
            self._data.pop(k, None)


@pytest.fixture
def mock_pool():
    pool = AsyncMock()
    pool.fetch = AsyncMock()
    pool.fetchrow = AsyncMock()
    pool.execute = AsyncMock()
    return pool


@pytest.fixture
def mock_redis():
    return FakeRedis()


@pytest.fixture
def service(mock_pool, mock_redis):
    import sys
    import os
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src", "generated_protos"))
    from book_catalog_server import BookCatalogService
    return BookCatalogService(pool=mock_pool, service_name="test", redis=mock_redis)