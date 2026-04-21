CREATE TABLE IF NOT EXISTS author (
    id   SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS book (
    isbn     BIGINT PRIMARY KEY,
    name     TEXT NOT NULL,
    url      TEXT,
    pub_year INT
);

CREATE TABLE IF NOT EXISTS book_author (
    book_isbn  BIGINT REFERENCES book(isbn)   ON DELETE CASCADE,
    author_id  INT    REFERENCES author(id)   ON DELETE CASCADE,
    PRIMARY KEY (book_isbn, author_id)
);

CREATE TABLE IF NOT EXISTS genre (
    id   SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS book_genre (
    book_isbn BIGINT REFERENCES book(isbn)  ON DELETE CASCADE,
    genre_id  INT    REFERENCES genre(id)   ON DELETE CASCADE,
    PRIMARY KEY (book_isbn, genre_id)
);

CREATE TABLE IF NOT EXISTS rating (
    book_isbn   TEXT PRIMARY KEY,
    num_ratings BIGINT,
    star_rating DOUBLE PRECISION
);