import pytest
from unittest.mock import AsyncMock
from types import SimpleNamespace
from datetime import datetime
from collections import defaultdict

from conftest import FakeRating, FakeBook


@pytest.fixture(autouse=True)
def patch_grpc(monkeypatch):
    monkeypatch.setattr("genre_analysis_server.rating_catalog_grpc", MagicMock())
    monkeypatch.setattr("genre_analysis_server.book_catalog_grpc", MagicMock())
    monkeypatch.setattr("genre_analysis_server.rating_catalog_pb2", MagicMock())


class TestGetGenres:
    async def test_returns_paginated_genres_sorted_by_rating(self, service, mock_pool):
        mock_pool.fetch.return_value = [
            {"genre_id": 1, "name": "Sci-Fi", "avg_rating": 4.5, "total_num_ratings": 100},
            {"genre_id": 2, "name": "Fantasy", "avg_rating": 3.0, "total_num_ratings": 200},
        ]
        from generated_protos.genre_service_pb2 import GetGenresRequest
        req = GetGenresRequest(sort_by=0, ascending=False, page_num=1, page_size=10)
        resp = await service.GetGenres(req, SimpleNamespace())
        assert len(resp.genres) == 2
        assert resp.genres[0].genre_name == "Sci-Fi"
        assert resp.genres[0].avg_rating == 4.5

    async def test_returns_empty_when_no_genres(self, service, mock_pool):
        mock_pool.fetch.return_value = []
        from generated_protos.genre_service_pb2 import GetGenresRequest
        req = GetGenresRequest(page_num=1, page_size=10)
        resp = await service.GetGenres(req, SimpleNamespace())
        assert resp.genres == []
        assert resp.total_items == 0

    async def test_pagination_clamps_page_size(self, service, mock_pool):
        mock_pool.fetch.return_value = [{"genre_id": i, "name": f"G{i}", "avg_rating": 0, "total_num_ratings": 0} for i in range(30)]
        from generated_protos.genre_service_pb2 import GetGenresRequest
        req = GetGenresRequest(page_num=1, page_size=100)
        resp = await service.GetGenres(req, SimpleNamespace())
        assert resp.page_size == 20

    async def test_sorts_by_popularity(self, service, mock_pool):
        mock_pool.fetch.return_value = [
            {"genre_id": 1, "name": "A", "avg_rating": 0, "total_num_ratings": 50},
            {"genre_id": 2, "name": "B", "avg_rating": 0, "total_num_ratings": 200},
        ]
        from generated_protos.genre_service_pb2 import GetGenresRequest
        req = GetGenresRequest(sort_by=1, ascending=False, page_num=1, page_size=10)
        resp = await service.GetGenres(req, SimpleNamespace())
        assert resp.genres[0].genre_id == 2

    async def test_cache_not_ready_falls_back(self, service, mock_pool):
        service.cache_ready = False
        mock_pool.fetch.side_effect = [
            [{"genre_id": 1, "name": "Sci-Fi"}],
            [{"book_isbn": 111}, {"book_isbn": 222}],
            [{"genre_id": 1, "book_isbn": 111}, {"genre_id": 1, "book_isbn": 222}],
        ]

        import sys
        sys.modules["genre_analysis_server"].rating_catalog_grpc = MagicMock()
        stub_mock = AsyncMock()
        stub_mock.GetRating.side_effect = [
            SimpleNamespace(rating=FakeRating(4.0, 100)),
            SimpleNamespace(rating=FakeRating(5.0, 50)),
        ]
        sys.modules["genre_analysis_server"].rating_catalog_grpc.RatingCatalogGrpcStub.return_value = stub_mock

        from generated_protos.genre_service_pb2 import GetGenresRequest
        req = GetGenresRequest(page_num=1, page_size=10)
        resp = await service.GetGenres(req, SimpleNamespace())
        assert len(resp.genres) == 1
        assert resp.genres[0].avg_rating == 4.5


class TestGetGenre:
    async def test_returns_genre_stats(self, service, mock_pool):
        mock_pool.fetchrow.return_value = {"genre_id": 1, "name": "Sci-Fi", "avg_rating": 4.5, "total_num_ratings": 100}
        from generated_protos.genre_service_pb2 import GetGenreRequest
        req = GetGenreRequest(genre_id=1)
        resp = await service.GetGenre(req, SimpleNamespace())
        assert resp.genre_name == "Sci-Fi"
        assert resp.avg_rating == 4.5

    async def test_not_found_returns_error(self, service, mock_pool):
        mock_pool.fetchrow.return_value = None
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import GetGenreRequest
        req = GetGenreRequest(genre_id=999)
        resp = await service.GetGenre(req, context)
        assert resp.genre_id == 0

    async def test_cache_not_ready_falls_back(self, service, mock_pool):
        service.cache_ready = False
        mock_pool.fetchrow.side_effect = [
            {"genre_id": 1, "name": "Sci-Fi"},
        ]
        mock_pool.fetch.return_value = [{"book_isbn": 111}]

        stub_mock = AsyncMock()
        stub_mock.GetRating.return_value = SimpleNamespace(rating=FakeRating(4.5, 200))
        service.rating_catalog_channel = MagicMock()
        import sys, unittest.mock
        with unittest.mock.patch.object(service, "_get_ratings_batch", new=AsyncMock(return_value={"111": FakeRating(4.5, 200)})):
            from generated_protos.genre_service_pb2 import GetGenreRequest
            req = GetGenreRequest(genre_id=1)
            resp = await service.GetGenre(req, SimpleNamespace())
            assert resp.avg_rating == 4.5


class TestAddGenre:
    async def test_adds_genre_successfully(self, service, mock_pool):
        mock_pool.fetchrow.return_value = {"genre_id": 5, "name": "New Genre"}
        from generated_protos.genre_service_pb2 import AddGenreRequest, Genre
        req = AddGenreRequest(genre=Genre(name="New Genre"))
        resp = await service.AddGenre(req, SimpleNamespace())
        assert resp.genre.genre_id == 5
        assert resp.genre.name == "New Genre"

    async def test_rejects_empty_name(self, service):
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import AddGenreRequest, Genre
        req = AddGenreRequest(genre=Genre(name=""))
        resp = await service.AddGenre(req, context)
        assert resp.genre.genre_id == 0


class TestUpdateGenre:
    async def test_updates_genre(self, service, mock_pool):
        mock_pool.fetchrow.return_value = {"genre_id": 1, "name": "Updated"}
        from generated_protos.genre_service_pb2 import UpdateGenreRequest, Genre
        req = UpdateGenreRequest(genre_id=1, genre=Genre(name="Updated"))
        resp = await service.UpdateGenre(req, SimpleNamespace())
        assert resp.genre.name == "Updated"

    async def test_not_found(self, service, mock_pool):
        mock_pool.fetchrow.return_value = None
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import UpdateGenreRequest, Genre
        req = UpdateGenreRequest(genre_id=999, genre=Genre(name="Ghost"))
        resp = await service.UpdateGenre(req, context)
        assert resp.genre.genre_id == 0


class TestDeleteGenre:
    async def test_deletes_genre(self, service, mock_pool):
        mock_pool.execute.return_value = "DELETE 1"
        from generated_protos.genre_service_pb2 import DeleteGenreRequest
        req = DeleteGenreRequest(genre_id=1)
        resp = await service.DeleteGenre(req, SimpleNamespace())
        assert resp is not None

    async def test_not_found(self, service, mock_pool):
        mock_pool.execute.return_value = "DELETE 0"
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import DeleteGenreRequest
        req = DeleteGenreRequest(genre_id=999)
        resp = await service.DeleteGenre(req, context)
        assert mock_pool.execute.called


class TestAddGenreToBook:
    async def test_adds_genre_to_book(self, service, mock_pool):
        from generated_protos.genre_service_pb2 import AddGenreToBookRequest, BookGenre
        req = AddGenreToBookRequest(bookGenre=BookGenre(isbn=123, genre_id=1))
        resp = await service.AddGenreToBook(req, SimpleNamespace())
        assert resp.bookGenre.isbn == 123

    async def test_rejects_empty_isbn(self, service):
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import AddGenreToBookRequest, BookGenre
        req = AddGenreToBookRequest(bookGenre=BookGenre(isbn=0, genre_id=0))
        resp = await service.AddGenreToBook(req, context)
        assert resp.bookGenre.isbn == 0


class TestRemoveGenreFromBook:
    async def test_removes_genre_from_book(self, service, mock_pool):
        mock_pool.execute.return_value = "DELETE 1"
        from generated_protos.genre_service_pb2 import RemoveGenreFromBookRequest, BookGenre
        req = RemoveGenreFromBookRequest(bookGenre=BookGenre(isbn=123, genre_id=1))
        resp = await service.RemoveGenreFromBook(req, SimpleNamespace())
        assert mock_pool.execute.called

    async def test_not_found(self, service, mock_pool):
        mock_pool.execute.return_value = "DELETE 0"
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import RemoveGenreFromBookRequest, BookGenre
        req = RemoveGenreFromBookRequest(bookGenre=BookGenre(isbn=999, genre_id=1))
        resp = await service.RemoveGenreFromBook(req, context)
        assert mock_pool.execute.called


class TestGetGenreGrowth:
    async def test_returns_yearly_growth_from_cache(self, service, mock_pool):
        mock_pool.fetchrow.return_value = {"genre_id": 1, "name": "Sci-Fi"}
        mock_pool.fetch.return_value = [
            {"year": 2020, "avg_rating": 4.0, "total_num_ratings": 50},
            {"year": 2021, "avg_rating": 4.5, "total_num_ratings": 100},
        ]
        from generated_protos.genre_service_pb2 import GenreGrowthRequest
        req = GenreGrowthRequest(genre_id=1, year_from=2020, year_to=2021)
        resp = await service.GetGenreGrowth(req, SimpleNamespace())
        assert len(resp.points) == 2
        assert resp.points[0].year == 2020
        assert resp.points[0].avg_rating == 4.0

    async def test_defaults_year_range(self, service, mock_pool):
        mock_pool.fetchrow.return_value = {"genre_id": 1, "name": "Sci-Fi"}
        mock_pool.fetch.return_value = []
        from generated_protos.genre_service_pb2 import GenreGrowthRequest
        req = GenreGrowthRequest(genre_id=1)
        resp = await service.GetGenreGrowth(req, SimpleNamespace())
        assert resp.points == []

    async def test_not_found(self, service, mock_pool):
        mock_pool.fetchrow.return_value = None
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import GenreGrowthRequest
        req = GenreGrowthRequest(genre_id=999)
        resp = await service.GetGenreGrowth(req, context)
        assert len(resp.points) == 0


class TestGetGenrePopularity:
    async def test_returns_yearly_popularity_from_cache(self, service, mock_pool):
        mock_pool.fetchrow.return_value = {"genre_id": 1, "name": "Sci-Fi"}
        mock_pool.fetch.return_value = [
            {"year": 2020, "total_num_ratings": 100, "book_count": 5},
            {"year": 2021, "total_num_ratings": 200, "book_count": 10},
        ]
        from generated_protos.genre_service_pb2 import GenrePopularityRequest
        req = GenrePopularityRequest(genre_id=1, year_from=2020, year_to=2021)
        resp = await service.GetGenrePopularity(req, SimpleNamespace())
        assert len(resp.points) == 2
        assert resp.points[0].total_num_ratings == 100

    async def test_not_found(self, service, mock_pool):
        mock_pool.fetchrow.return_value = None
        context = SimpleNamespace()
        context.set_code = MagicMock()
        context.set_details = MagicMock()
        from generated_protos.genre_service_pb2 import GenrePopularityRequest
        req = GenrePopularityRequest(genre_id=999)
        resp = await service.GetGenrePopularity(req, context)
        assert resp.total_num_ratings == 0


from unittest.mock import MagicMock