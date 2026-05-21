from types import SimpleNamespace


class FakeCatalogCache:
    def __init__(self, books=None, ratings_by_isbn=None, genres_by_isbn=None):
        self._books = books or []
        self._ratings_by_isbn = ratings_by_isbn or {}
        self._genres_by_isbn = genres_by_isbn or {}

    async def get_books(self):
        return self._books

    async def get_ratings_by_isbn(self):
        return self._ratings_by_isbn

    async def get_genres_by_isbn(self):
        return self._genres_by_isbn


def book(isbn, name, pub_year=None):
    return SimpleNamespace(isbn=isbn, name=name, pub_year=pub_year)


def genre(genre_id, genre_name):
    return SimpleNamespace(genre_id=genre_id, genre_name=genre_name)
