import pytest
import json
from unittest.mock import AsyncMock, MagicMock, patch
from types import SimpleNamespace


SAMPLE_BOOK_ROW = {"isbn": 1, "name": "Test Book", "url": "", "summary_clean": "", "pub_year": 2023}


class TestGetBooks:
    async def test_returns_paginated_books(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = {"cnt": 1}
        mock_pool.fetch.return_value = [SAMPLE_BOOK_ROW]

        from generated_protos.book_catalog_pb2 import GetBooksRequest
        req = GetBooksRequest(page_num=1, page_size=10)
        resp = await service.GetBooks(req, SimpleNamespace())
        assert len(resp.books) == 1
        assert resp.books[0].isbn == 1
        assert resp.total_items == 1

    async def test_filters_by_author(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = {"cnt": 1}
        mock_pool.fetch.return_value = [SAMPLE_BOOK_ROW]

        from generated_protos.book_catalog_pb2 import GetBooksRequest
        req = GetBooksRequest(page_num=1, page_size=10, author_id=5)
        resp = await service.GetBooks(req, SimpleNamespace())
        assert len(resp.books) == 1

    async def test_returns_empty_when_no_books(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = {"cnt": 0}
        mock_pool.fetch.return_value = []

        from generated_protos.book_catalog_pb2 import GetBooksRequest
        req = GetBooksRequest(page_num=1, page_size=10)
        resp = await service.GetBooks(req, SimpleNamespace())
        assert resp.books == []
        assert resp.total_items == 0

    async def test_clamps_invalid_page(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = {"cnt": 0}
        mock_pool.fetch.return_value = []

        from generated_protos.book_catalog_pb2 import GetBooksRequest
        req = GetBooksRequest(page_num=0, page_size=0)
        resp = await service.GetBooks(req, SimpleNamespace())
        assert resp.page_num == 1
        assert resp.page_size == 10


class TestGetBook:
    async def test_returns_book_by_isbn(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = SAMPLE_BOOK_ROW

        from generated_protos.book_catalog_pb2 import GetBookRequest
        req = GetBookRequest(isbn=1)
        resp = await service.GetBook(req, SimpleNamespace())
        assert resp.book.isbn == 1
        assert resp.book.name == "Test Book"

    async def test_not_found(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = None
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()

        from generated_protos.book_catalog_pb2 import GetBookRequest
        req = GetBookRequest(isbn=999)
        resp = await service.GetBook(req, context)
        assert resp.book.isbn == 0

    async def test_uses_cache(self, service, mock_pool, mock_redis):
        cached = json.dumps(SAMPLE_BOOK_ROW)
        await mock_redis.setex("book:1", 3600, cached)

        from generated_protos.book_catalog_pb2 import GetBookRequest
        req = GetBookRequest(isbn=1)
        resp = await service.GetBook(req, SimpleNamespace())
        assert resp.book.name == "Test Book"
        mock_pool.fetchrow.assert_not_called()


class TestAddBook:
    async def test_adds_book_successfully(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = SAMPLE_BOOK_ROW

        from generated_protos.book_catalog_pb2 import AddBookRequest, BookAdd
        req = AddBookRequest(book=BookAdd(isbn=1, name="Test Book", pub_year=2023))
        resp = await service.AddBook(req, SimpleNamespace())
        assert resp.book.isbn == 1

    async def test_rejects_missing_name(self, service, mock_pool, mock_redis):
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()

        from generated_protos.book_catalog_pb2 import AddBookRequest, BookAdd
        req = AddBookRequest(book=BookAdd(isbn=1))
        resp = await service.AddBook(req, context)
        assert resp.book.isbn == 0


class TestUpdateBook:
    async def test_updates_book(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = SAMPLE_BOOK_ROW

        from generated_protos.book_catalog_pb2 import UpdateBookRequest, BookAdd
        req = UpdateBookRequest(isbn=1, book=BookAdd(name="Updated", pub_year=2024))
        resp = await service.UpdateBook(req, SimpleNamespace())
        assert resp.book.name == "Updated"

    async def test_not_found(self, service, mock_pool, mock_redis):
        mock_pool.fetchrow.return_value = None
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()

        from generated_protos.book_catalog_pb2 import UpdateBookRequest, BookAdd
        req = UpdateBookRequest(isbn=999, book=BookAdd(name="Ghost", pub_year=2023))
        resp = await service.UpdateBook(req, context)
        assert resp.book.isbn == 0


class TestDeleteBook:
    async def test_deletes_book(self, service, mock_pool, mock_redis):
        mock_pool.execute.return_value = "DELETE 1"

        from generated_protos.book_catalog_pb2 import DeleteBookRequest
        req = DeleteBookRequest(isbn=1)
        resp = await service.DeleteBook(req, SimpleNamespace())
        assert mock_pool.execute.called

    async def test_not_found(self, service, mock_pool, mock_redis):
        mock_pool.execute.return_value = "DELETE 0"
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()

        from generated_protos.book_catalog_pb2 import DeleteBookRequest
        req = DeleteBookRequest(isbn=999)
        resp = await service.DeleteBook(req, context)
        assert mock_pool.execute.called