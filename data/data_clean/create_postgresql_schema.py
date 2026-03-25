from __future__ import annotations

import os
import re
import time
from pathlib import Path
from typing import Optional, List, Tuple

import pandas as pd
import psycopg2
from psycopg2.extras import execute_values


CSV_DIR = Path(os.getenv("CSV_DIR", "../dataset/csvs"))
CHUNKSIZE = int(os.getenv("CHUNKSIZE", "200000"))

PGHOST = os.getenv("PGHOST", "localhost")
PGPORT = int(os.getenv("PGPORT", "5432"))
PGUSER = os.getenv("PGUSER", "bookcloud")
PGPASSWORD = os.getenv("PGPASSWORD", "bookcloud")
PGDATABASE = os.getenv("PGDATABASE", "bookcloud_db")

SCHEMA = os.getenv("PGSCHEMA", "bookcloud")

BOOK_KEEP = os.getenv("BOOK_KEEP", "first").lower()

CSV_LIMIT = int(os.getenv("CSV_LIMIT", "0"))
CSV_ONLY = os.getenv("CSV_ONLY", "").strip()
CSV_SKIP = os.getenv("CSV_SKIP", "").strip()
MAX_CHUNKS_PER_FILE = int(os.getenv("MAX_CHUNKS_PER_FILE", "0"))


def _py(v):
    """Convert pandas/numpy scalars to native Python types."""
    if v is None:
        return None

    try:
        if pd.isna(v):
            return None
    except Exception:
        pass

    if hasattr(v, "item"):
        try:
            return v.item()
        except Exception:
            pass

    return v


def _clean_text(s: str) -> str:
    s = str(s) if s is not None else ""
    s = s.strip()
    s = re.sub(r"\s+", " ", s)
    return s


def _split_multi_values(value: str) -> list[str]:
    if value is None or (isinstance(value, float) and pd.isna(value)):
        return []

    text = _clean_text(value)
    if not text:
        return []

    seps = ["|", ";", " / ", "/", " , ", ", "]
    for sep in seps:
        if sep in text:
            parts = [p.strip() for p in text.split(sep)]
            parts = [p for p in parts if p]

            seen = set()
            out = []
            for p in parts:
                key = p.casefold()
                if key not in seen:
                    seen.add(key)
                    out.append(p)
            return out

    return [text]


def _parse_list_string(value: str) -> list[str]:
    if value is None or (isinstance(value, float) and pd.isna(value)):
        return []

    text = _clean_text(value)
    if not text:
        return []

    if text.startswith("[") and text.endswith("]"):
        inner = text[1:-1].strip()
        tokens = re.findall(r"'([^']+)'|\"([^\"]+)\"", inner)

        items = []
        for a, b in tokens:
            it = a if a else b
            it = _clean_text(it)
            if it:
                items.append(it)

        if items:
            seen = set()
            out = []
            for it in items:
                key = it.casefold()
                if key not in seen:
                    seen.add(key)
                    out.append(it)
            return out

        return _split_multi_values(inner)

    return _split_multi_values(text)


def _normalize_isbn(isbn) -> Optional[str]:
    if isbn is None or (isinstance(isbn, float) and pd.isna(isbn)):
        return None
    s = re.sub(r"[^0-9Xx]", "", str(isbn)).upper().strip()
    return s if s else None


def _isbn_to_int(isbn_clean) -> Optional[int]:
    if isbn_clean is None or (isinstance(isbn_clean, float) and pd.isna(isbn_clean)):
        return None

    s = str(isbn_clean).strip().upper()
    if not s:
        return None
    if "X" in s:
        return None
    if not s.isdigit():
        return None

    try:
        return int(s)
    except ValueError:
        return None


def ddl_sql(schema: str) -> str:
    """Create schema and tables."""
    return f"""
CREATE SCHEMA IF NOT EXISTS {schema};

CREATE TABLE IF NOT EXISTS {schema}.book (
  isbn          BIGINT PRIMARY KEY,
  name          TEXT NOT NULL,
  url           TEXT,
  summary_clean TEXT,
  pub_year      INT
);

CREATE TABLE IF NOT EXISTS {schema}.rating (
  book_isbn    BIGINT PRIMARY KEY REFERENCES {schema}.book(isbn) ON DELETE CASCADE,
  star_rating  DOUBLE PRECISION,
  num_ratings  BIGINT
);

CREATE TABLE IF NOT EXISTS {schema}.author (
  author_id    BIGSERIAL PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS {schema}.genre (
  genre_id     BIGSERIAL PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS {schema}.book_author (
  book_isbn BIGINT NOT NULL REFERENCES {schema}.book(isbn) ON DELETE CASCADE,
  author_id BIGINT NOT NULL REFERENCES {schema}.author(author_id) ON DELETE CASCADE,
  PRIMARY KEY (book_isbn, author_id)
);

CREATE TABLE IF NOT EXISTS {schema}.book_genre (
  book_isbn BIGINT NOT NULL REFERENCES {schema}.book(isbn) ON DELETE CASCADE,
  genre_id  BIGINT NOT NULL REFERENCES {schema}.genre(genre_id) ON DELETE CASCADE,
  PRIMARY KEY (book_isbn, genre_id)
);

CREATE TABLE IF NOT EXISTS {schema}.staging_author_name (
  name TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS {schema}.staging_genre_name (
  name TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS {schema}.staging_book_author_name (
  book_isbn BIGINT NOT NULL,
  author_name TEXT NOT NULL,
  PRIMARY KEY (book_isbn, author_name)
);

CREATE TABLE IF NOT EXISTS {schema}.staging_book_genre_name (
  book_isbn BIGINT NOT NULL,
  genre_name TEXT NOT NULL,
  PRIMARY KEY (book_isbn, genre_name)
);
""".strip()


def finalize_sql(schema: str) -> str:
    """Load dimension and relation tables from staging."""
    return f"""
INSERT INTO {schema}.author(name)
SELECT s.name
FROM {schema}.staging_author_name s
ON CONFLICT (name) DO NOTHING;

INSERT INTO {schema}.genre(name)
SELECT s.name
FROM {schema}.staging_genre_name s
ON CONFLICT (name) DO NOTHING;

INSERT INTO {schema}.book_author(book_isbn, author_id)
SELECT s.book_isbn, a.author_id
FROM {schema}.staging_book_author_name s
JOIN {schema}.author a ON a.name = s.author_name
ON CONFLICT DO NOTHING;

INSERT INTO {schema}.book_genre(book_isbn, genre_id)
SELECT s.book_isbn, g.genre_id
FROM {schema}.staging_book_genre_name s
JOIN {schema}.genre g ON g.name = s.genre_name
ON CONFLICT DO NOTHING;
""".strip()


def truncate_staging_sql(schema: str) -> str:
    """Clear staging tables."""
    return f"""
TRUNCATE TABLE
  {schema}.staging_author_name,
  {schema}.staging_genre_name,
  {schema}.staging_book_author_name,
  {schema}.staging_book_genre_name;
""".strip()


def deduplicate_books_sql(schema: str) -> str:
    """Remove logical duplicates and keep the preferred ISBN."""
    return f"""
WITH author_sets AS (
    SELECT
        ba.book_isbn,
        STRING_AGG(a.name, '|' ORDER BY a.name) AS authors_key
    FROM {schema}.book_author ba
    JOIN {schema}.author a ON a.author_id = ba.author_id
    GROUP BY ba.book_isbn
),
book_fingerprint AS (
    SELECT
        b.isbn,
        b.name,
        b.pub_year,
        ROUND(r.star_rating::numeric, 4) AS star_rating_norm,
        r.num_ratings,
        COALESCE(au.authors_key, '') AS authors_key,
        CASE
            WHEN LENGTH(b.isbn::text) = 13 THEN 0
            ELSE 1
        END AS isbn_priority
    FROM {schema}.book b
    JOIN {schema}.rating r ON r.book_isbn = b.isbn
    LEFT JOIN author_sets au ON au.book_isbn = b.isbn
),
ranked AS (
    SELECT
        isbn,
        ROW_NUMBER() OVER (
            PARTITION BY name, pub_year, star_rating_norm, num_ratings, authors_key
            ORDER BY isbn_priority ASC, isbn ASC
        ) AS rn
    FROM book_fingerprint
),
dupes AS (
    SELECT isbn
    FROM ranked
    WHERE rn > 1
)
DELETE FROM {schema}.book b
USING dupes d
WHERE b.isbn = d.isbn;
""".strip()


def _connect():
    return psycopg2.connect(
        host=PGHOST,
        port=PGPORT,
        user=PGUSER,
        password=PGPASSWORD,
        dbname=PGDATABASE,
    )


def _execute_ddl(conn, schema: str) -> None:
    with conn.cursor() as cur:
        cur.execute(ddl_sql(schema))
    conn.commit()


def _bulk_insert_book(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    if BOOK_KEEP == "last":
        sql = f"""
        INSERT INTO {schema}.book(isbn, name, url, summary_clean, pub_year)
        VALUES %s
        ON CONFLICT (isbn) DO UPDATE SET
          name = EXCLUDED.name,
          url = EXCLUDED.url,
          summary_clean = EXCLUDED.summary_clean,
          pub_year = EXCLUDED.pub_year;
        """
    else:
        sql = f"""
        INSERT INTO {schema}.book(isbn, name, url, summary_clean, pub_year)
        VALUES %s
        ON CONFLICT (isbn) DO NOTHING;
        """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _bulk_upsert_rating(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    sql = f"""
    INSERT INTO {schema}.rating(book_isbn, star_rating, num_ratings)
    VALUES %s
    ON CONFLICT (book_isbn) DO UPDATE SET
      star_rating = COALESCE(EXCLUDED.star_rating, {schema}.rating.star_rating),
      num_ratings = COALESCE(EXCLUDED.num_ratings, {schema}.rating.num_ratings);
    """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _bulk_insert_staging_single_col(conn, schema: str, table: str, names: List[str]) -> int:
    if not names:
        return 0

    sql = f"INSERT INTO {schema}.{table}(name) VALUES %s ON CONFLICT (name) DO NOTHING;"
    rows = [(n,) for n in names]

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=20000)

    return len(rows)


def _bulk_insert_staging_pairs(conn, schema: str, table: str, rows: List[Tuple[int, str]]) -> int:
    if not rows:
        return 0

    if table == "staging_book_author_name":
        sql = f"""
        INSERT INTO {schema}.{table}(book_isbn, author_name)
        VALUES %s
        ON CONFLICT (book_isbn, author_name) DO NOTHING;
        """
    else:
        sql = f"""
        INSERT INTO {schema}.{table}(book_isbn, genre_name)
        VALUES %s
        ON CONFLICT (book_isbn, genre_name) DO NOTHING;
        """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=20000)

    return len(rows)


def _iter_csv_files(folder: Path) -> List[Path]:
    files = sorted(folder.glob("*.csv"))
    if not files:
        raise FileNotFoundError(f"No CSV files found in {folder.resolve()}")

    if CSV_ONLY:
        only_set = {x.strip() for x in CSV_ONLY.split(",") if x.strip()}
        files = [f for f in files if f.name in only_set]

    if CSV_SKIP:
        skip_set = {x.strip() for x in CSV_SKIP.split(",") if x.strip()}
        files = [f for f in files if f.name not in skip_set]

    if CSV_LIMIT and CSV_LIMIT > 0:
        files = files[:CSV_LIMIT]

    if not files:
        raise FileNotFoundError("No CSV files left after filtering.")

    return files


def _require_cols(df: pd.DataFrame, cols: List[str], filename: str) -> None:
    missing = [c for c in cols if c not in df.columns]
    if missing:
        raise ValueError(f"[{filename}] missing columns: {missing}")


def import_csvs_to_postgres(schema: str = SCHEMA) -> None:
    start_time = time.perf_counter()

    print(f"Connecting to PostgreSQL at {PGHOST}:{PGPORT}, database={PGDATABASE}")
    conn = _connect()
    conn.autocommit = False

    try:
        print(f"Ensuring schema and tables for '{schema}'")
        _execute_ddl(conn, schema)

        csv_files = _iter_csv_files(CSV_DIR)
        print(f"CSV directory: {CSV_DIR.resolve()}")
        print(f"Files to import: {len(csv_files)}")
        print(f"Chunk size: {CHUNKSIZE}")

        total_files = len(csv_files)

        for file_index, fp in enumerate(csv_files, start=1):
            file_start = time.perf_counter()
            print(f"[{file_index}/{total_files}] Importing file: {fp.name}")

            chunk_iter = pd.read_csv(fp, low_memory=False, chunksize=CHUNKSIZE)

            for chunk_index, chunk in enumerate(chunk_iter, start=1):
                if MAX_CHUNKS_PER_FILE and chunk_index > MAX_CHUNKS_PER_FILE:
                    print(
                        f"  Reached MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE} for {fp.name}. "
                        "Skipping remaining chunks."
                    )
                    break

                print(f"  Processing chunk {chunk_index} from {fp.name}")

                _require_cols(
                    chunk,
                    [
                        "name", "url", "summary_clean", "pub_year",
                        "isbn_clean", "star_rating", "num_ratings",
                        "author", "genres",
                    ],
                    fp.name
                )

                original_rows = len(chunk)

                isbn = chunk["isbn_clean"].map(_normalize_isbn).map(_isbn_to_int)
                chunk = chunk.assign(isbn=isbn).dropna(subset=["isbn"])

                valid_rows = len(chunk)
                dropped_rows = original_rows - valid_rows

                if chunk.empty:
                    print(f"  Chunk {chunk_index} skipped: no valid ISBN rows")
                    continue

                chunk["isbn"] = pd.to_numeric(chunk["isbn"], errors="coerce").astype("int64")

                book_df = pd.DataFrame({
                    "isbn": chunk["isbn"],
                    "name": chunk["name"].map(_clean_text).replace("", pd.NA),
                    "url": chunk["url"].map(_clean_text),
                    "summary_clean": chunk["summary_clean"].astype(str).map(_clean_text),
                    "pub_year": pd.to_numeric(chunk["pub_year"], errors="coerce").astype("Int64"),
                }).dropna(subset=["name"])

                book_rows = [
                    (_py(isbn), _py(name), _py(url), _py(summary), _py(pub_year))
                    for isbn, name, url, summary, pub_year in book_df.itertuples(index=False, name=None)
                ]

                inserted_books = _bulk_insert_book(conn, schema, book_rows)

                rating_df = pd.DataFrame({
                    "book_isbn": chunk["isbn"],
                    "star_rating": pd.to_numeric(chunk["star_rating"], errors="coerce"),
                    "num_ratings": pd.to_numeric(chunk["num_ratings"], errors="coerce").astype("Int64"),
                })

                rating_df = (
                    rating_df.sort_values("book_isbn")
                    .groupby("book_isbn", as_index=False)
                    .agg(
                        star_rating=("star_rating", "first"),
                        num_ratings=("num_ratings", "first"),
                    )
                )

                rating_rows = [
                    (_py(book_isbn), _py(star_rating), _py(num_ratings))
                    for book_isbn, star_rating, num_ratings in rating_df.itertuples(index=False, name=None)
                ]

                upserted_ratings = _bulk_upsert_rating(conn, schema, rating_rows)

                tmp_a = chunk[["isbn", "author"]].copy()
                tmp_a["author_list"] = tmp_a["author"].apply(_parse_list_string)
                tmp_a = tmp_a.explode("author_list")
                tmp_a["author_list"] = tmp_a["author_list"].map(_clean_text)
                tmp_a = tmp_a[tmp_a["author_list"].notna() & (tmp_a["author_list"] != "")]

                author_names = tmp_a["author_list"].drop_duplicates().tolist()
                inserted_author_names = _bulk_insert_staging_single_col(
                    conn, schema, "staging_author_name", author_names
                )

                ba_pairs = list({(int(_py(r.isbn)), str(_py(r.author_list))) for r in tmp_a.itertuples(index=False)})
                inserted_ba_pairs = _bulk_insert_staging_pairs(
                    conn, schema, "staging_book_author_name", ba_pairs
                )

                tmp_g = chunk[["isbn", "genres"]].copy()
                tmp_g["genre_list"] = tmp_g["genres"].apply(_parse_list_string)
                tmp_g = tmp_g.explode("genre_list")
                tmp_g["genre_list"] = tmp_g["genre_list"].map(_clean_text)
                tmp_g = tmp_g[tmp_g["genre_list"].notna() & (tmp_g["genre_list"] != "")]

                genre_names = tmp_g["genre_list"].drop_duplicates().tolist()
                inserted_genre_names = _bulk_insert_staging_single_col(
                    conn, schema, "staging_genre_name", genre_names
                )

                bg_pairs = list({(int(_py(r.isbn)), str(_py(r.genre_list))) for r in tmp_g.itertuples(index=False)})
                inserted_bg_pairs = _bulk_insert_staging_pairs(
                    conn, schema, "staging_book_genre_name", bg_pairs
                )

                conn.commit()

                print(
                    f"  Chunk {chunk_index} committed | "
                    f"input_rows={original_rows} | valid_isbn_rows={valid_rows} | dropped_invalid_isbn={dropped_rows} | "
                    f"book_rows={inserted_books} | rating_rows={upserted_ratings} | "
                    f"author_names={inserted_author_names} | genre_names={inserted_genre_names} | "
                    f"book_author_pairs={inserted_ba_pairs} | book_genre_pairs={inserted_bg_pairs}"
                )

            elapsed_file = time.perf_counter() - file_start
            print(f"Finished file: {fp.name} in {elapsed_file:.2f}s")

        print("Finalizing dimensions and relations")
        with conn.cursor() as cur:
            cur.execute(finalize_sql(schema))
        conn.commit()

        print("Removing logical duplicate books")
        with conn.cursor() as cur:
            cur.execute(deduplicate_books_sql(schema))
        conn.commit()

        print("Cleaning staging tables")
        with conn.cursor() as cur:
            cur.execute(truncate_staging_sql(schema))
        conn.commit()

        elapsed = time.perf_counter() - start_time
        print(f"Import finished in {elapsed:.2f}s")

    finally:
        conn.close()
        print("PostgreSQL connection closed")


if __name__ == "__main__":
    import_csvs_to_postgres()