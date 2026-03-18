CREATE SCHEMA IF NOT EXISTS bookcloud;

CREATE TABLE IF NOT EXISTS bookcloud.book (
  isbn          BIGINT PRIMARY KEY,
  name          TEXT NOT NULL,
  url           TEXT,
  summary_clean TEXT,
  pub_year      INT
);

CREATE TABLE IF NOT EXISTS bookcloud.rating (
  book_isbn    BIGINT PRIMARY KEY REFERENCES bookcloud.book(isbn) ON DELETE CASCADE,
  star_rating  DOUBLE PRECISION,
  num_ratings  BIGINT
);

CREATE TABLE IF NOT EXISTS bookcloud.author (
  author_id    BIGSERIAL PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS bookcloud.genre (
  genre_id     BIGSERIAL PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS bookcloud.book_author (
  book_isbn BIGINT NOT NULL REFERENCES bookcloud.book(isbn) ON DELETE CASCADE,
  author_id BIGINT NOT NULL REFERENCES bookcloud.author(author_id) ON DELETE CASCADE,
  PRIMARY KEY (book_isbn, author_id)
);

CREATE TABLE IF NOT EXISTS bookcloud.book_genre (
  book_isbn BIGINT NOT NULL REFERENCES bookcloud.book(isbn) ON DELETE CASCADE,
  genre_id  BIGINT NOT NULL REFERENCES bookcloud.genre(genre_id) ON DELETE CASCADE,
  PRIMARY KEY (book_isbn, genre_id)
);