from __future__ import annotations

import os
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import pandas as pd
import psycopg2
from psycopg2.extras import execute_values


def _env_first(*names: str, default: Optional[str] = None) -> Optional[str]:
    for name in names:
        value = os.getenv(name)
        if value is not None and str(value).strip() != "":
            return value
    return default


CSV_DIR = Path(_env_first("CSV_DIR", default="./normalized_out"))
CHUNKSIZE = int(_env_first("CHUNKSIZE", default="200000"))
MAX_CHUNKS_PER_FILE = int(_env_first("MAX_CHUNKS_PER_FILE", default="0"))

PGHOST = _env_first("PGHOST", "POSTGRES_HOST", default="localhost")
PGPORT = int(_env_first("PGPORT", "POSTGRES_PORT", default="5432"))
PGUSER = _env_first("PGUSER", "POSTGRES_USER", default="bookcloud")
PGPASSWORD = _env_first("PGPASSWORD", "POSTGRES_PASSWORD", default="bookcloud")
PGDATABASE = _env_first("PGDATABASE", "POSTGRES_DB", default="bookcloud_db")
SCHEMA = _env_first("PGSCHEMA", "POSTGRES_SCHEMA", default="public")

SERVICE = _env_first("SERVICE", default="all").strip()


SERVICE_FILES: Dict[str, List[str]] = {
    "bookcatalog": ["book.csv"],
    "ratingcatalog": ["rating.csv"],
    "authorcatalog": ["author.csv", "book_author.csv"],
    "genreanalysis": ["genre.csv", "book_genre.csv"],
    "all": ["book.csv", "rating.csv", "author.csv", "book_author.csv", "genre.csv", "book_genre.csv"],
}

SERVICE_TABLES: Dict[str, List[str]] = {
    "bookcatalog": ["book"],
    "ratingcatalog": ["rating"],
    "authorcatalog": ["author", "book_author"],
    "genreanalysis": ["genre", "book_genre"],
    "all": ["book", "rating", "author", "book_author", "genre", "book_genre"],
}


def normalize_service_name(service: str) -> str:
    if not service:
        return "all"

    key = service.strip().lower().replace("-", "").replace("_", "")
    aliases = {
        "bookcatalog": "bookcatalog",
        "book": "bookcatalog",

        "ratingcatalog": "ratingcatalog",
        "rating": "ratingcatalog",

        "authorcatalog": "authorcatalog",
        "author": "authorcatalog",
        "authors": "authorcatalog",

        "genreanalysis": "genreanalysis",
        "genre": "genreanalysis",
        "genres": "genreanalysis",

        "all": "all",
    }

    if key not in aliases:
        valid = ", ".join(sorted(SERVICE_FILES.keys()))
        raise ValueError(f"Invalid service '{service}'. Valid values: {valid}")

    return aliases[key]


def _py(v):
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


def _connect():
    return psycopg2.connect(
        host=PGHOST,
        port=PGPORT,
        user=PGUSER,
        password=PGPASSWORD,
        dbname=PGDATABASE,
    )


def ddl_sql(schema: str, service: str) -> str:
    service = normalize_service_name(service)
    stmts = [f"CREATE SCHEMA IF NOT EXISTS {schema};"]

    if service in ("bookcatalog", "all"):
        stmts.append(f"""
        CREATE TABLE IF NOT EXISTS {schema}.book (
            isbn BIGINT PRIMARY KEY,
            name TEXT NOT NULL,
            url TEXT,
            summary_clean TEXT,
            pub_year INT
        );
        """)

    if service in ("ratingcatalog", "all"):
        stmts.append(f"""
        CREATE TABLE IF NOT EXISTS {schema}.rating (
            book_isbn BIGINT PRIMARY KEY,
            star_rating DOUBLE PRECISION,
            num_ratings BIGINT
        );
        """)

    if service in ("authorcatalog", "all"):
        stmts.append(f"""
        CREATE TABLE IF NOT EXISTS {schema}.author (
            author_id BIGINT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );
        """)

        stmts.append(f"""
        CREATE TABLE IF NOT EXISTS {schema}.book_author (
            book_isbn BIGINT NOT NULL,
            author_id BIGINT NOT NULL,
            PRIMARY KEY (book_isbn, author_id)
        );
        """)

    if service in ("genreanalysis", "all"):
        stmts.append(f"""
        CREATE TABLE IF NOT EXISTS {schema}.genre (
            genre_id BIGINT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );
        """)

        stmts.append(f"""
        CREATE TABLE IF NOT EXISTS {schema}.book_genre (
            book_isbn BIGINT NOT NULL,
            genre_id BIGINT NOT NULL,
            PRIMARY KEY (book_isbn, genre_id)
        );
        """)

    return "\n".join(stmt.strip() for stmt in stmts if stmt.strip())


def drop_tables_sql(schema: str, service: str) -> str:
    service = normalize_service_name(service)
    tables = SERVICE_TABLES[service]

    ordered_drop = []
    for table in ("book_author", "book_genre", "rating", "author", "genre", "book"):
        if table in tables:
            ordered_drop.append(f"DROP TABLE IF EXISTS {schema}.{table} CASCADE;")

    return "\n".join(ordered_drop)


def _execute_sql(conn, sql: str) -> None:
    if not sql.strip():
        return
    with conn.cursor() as cur:
        cur.execute(sql)
    conn.commit()


def _require_cols(df: pd.DataFrame, cols: List[str], filename: str) -> None:
    missing = [c for c in cols if c not in df.columns]
    if missing:
        raise ValueError(f"[{filename}] missing columns: {missing}")


def _iter_service_csv_files(folder: Path, service: str) -> List[Path]:
    service = normalize_service_name(service)
    expected_files = SERVICE_FILES[service]

    files = []
    for filename in expected_files:
        fp = folder / filename
        if not fp.exists():
            raise FileNotFoundError(f"Expected file not found for service '{service}': {fp.resolve()}")
        files.append(fp)

    return files


def _bulk_insert_book(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    sql = f"""
    INSERT INTO {schema}.book (isbn, name, url, summary_clean, pub_year)
    VALUES %s
    ON CONFLICT (isbn) DO UPDATE SET
        name = EXCLUDED.name,
        url = EXCLUDED.url,
        summary_clean = EXCLUDED.summary_clean,
        pub_year = EXCLUDED.pub_year;
    """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _bulk_insert_rating(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    sql = f"""
    INSERT INTO {schema}.rating (book_isbn, star_rating, num_ratings)
    VALUES %s
    ON CONFLICT (book_isbn) DO UPDATE SET
        star_rating = EXCLUDED.star_rating,
        num_ratings = EXCLUDED.num_ratings;
    """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _bulk_insert_author(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    sql = f"""
    INSERT INTO {schema}.author (author_id, name)
    VALUES %s
    ON CONFLICT (author_id) DO UPDATE SET
        name = EXCLUDED.name;
    """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _bulk_insert_book_author(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    sql = f"""
    INSERT INTO {schema}.book_author (book_isbn, author_id)
    VALUES %s
    ON CONFLICT (book_isbn, author_id) DO NOTHING;
    """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _bulk_insert_genre(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    sql = f"""
    INSERT INTO {schema}.genre (genre_id, name)
    VALUES %s
    ON CONFLICT (genre_id) DO UPDATE SET
        name = EXCLUDED.name;
    """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _bulk_insert_book_genre(conn, schema: str, rows: List[Tuple]) -> int:
    if not rows:
        return 0

    sql = f"""
    INSERT INTO {schema}.book_genre (book_isbn, genre_id)
    VALUES %s
    ON CONFLICT (book_isbn, genre_id) DO NOTHING;
    """

    with conn.cursor() as cur:
        execute_values(cur, sql, rows, page_size=10000)

    return len(rows)


def _import_book_csv(conn, schema: str, fp: Path) -> None:
    file_start = time.perf_counter()
    print(f"Importing {fp.name} ...")

    chunk_iter = pd.read_csv(fp, low_memory=False, chunksize=CHUNKSIZE)

    for chunk_index, chunk in enumerate(chunk_iter, start=1):
        if MAX_CHUNKS_PER_FILE and chunk_index > MAX_CHUNKS_PER_FILE:
            print(f"  Reached MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE} in {fp.name}")
            break

        _require_cols(chunk, ["isbn", "name", "url", "summary_clean", "pub_year"], fp.name)

        chunk["isbn"] = pd.to_numeric(chunk["isbn"], errors="coerce").astype("Int64")
        chunk["pub_year"] = pd.to_numeric(chunk["pub_year"], errors="coerce").astype("Int64")
        chunk = chunk.dropna(subset=["isbn", "name"])

        rows = [
            (
                int(_py(isbn)),
                str(_py(name)).strip(),
                _py(url),
                _py(summary_clean),
                _py(pub_year),
            )
            for isbn, name, url, summary_clean, pub_year in chunk.itertuples(index=False, name=None)
        ]

        count = _bulk_insert_book(conn, schema, rows)
        conn.commit()
        print(f"  Chunk {chunk_index} committed | rows={count}")

    print(f"Finished {fp.name} in {time.perf_counter() - file_start:.2f}s")


def _import_rating_csv(conn, schema: str, fp: Path) -> None:
    file_start = time.perf_counter()
    print(f"Importing {fp.name} ...")

    chunk_iter = pd.read_csv(fp, low_memory=False, chunksize=CHUNKSIZE)

    for chunk_index, chunk in enumerate(chunk_iter, start=1):
        if MAX_CHUNKS_PER_FILE and chunk_index > MAX_CHUNKS_PER_FILE:
            print(f"  Reached MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE} in {fp.name}")
            break

        _require_cols(chunk, ["book_isbn", "star_rating", "num_ratings"], fp.name)

        chunk["book_isbn"] = pd.to_numeric(chunk["book_isbn"], errors="coerce").astype("Int64")
        chunk["star_rating"] = pd.to_numeric(chunk["star_rating"], errors="coerce")
        chunk["num_ratings"] = pd.to_numeric(chunk["num_ratings"], errors="coerce").astype("Int64")
        chunk = chunk.dropna(subset=["book_isbn"])

        rows = [
            (
                int(_py(book_isbn)),
                _py(star_rating),
                _py(num_ratings),
            )
            for book_isbn, star_rating, num_ratings in chunk.itertuples(index=False, name=None)
        ]

        count = _bulk_insert_rating(conn, schema, rows)
        conn.commit()
        print(f"  Chunk {chunk_index} committed | rows={count}")

    print(f"Finished {fp.name} in {time.perf_counter() - file_start:.2f}s")


def _import_author_csv(conn, schema: str, fp: Path) -> None:
    file_start = time.perf_counter()
    print(f"Importing {fp.name} ...")

    chunk_iter = pd.read_csv(fp, low_memory=False, chunksize=CHUNKSIZE)

    for chunk_index, chunk in enumerate(chunk_iter, start=1):
        if MAX_CHUNKS_PER_FILE and chunk_index > MAX_CHUNKS_PER_FILE:
            print(f"  Reached MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE} in {fp.name}")
            break

        _require_cols(chunk, ["author_id", "name"], fp.name)

        chunk["author_id"] = pd.to_numeric(chunk["author_id"], errors="coerce").astype("Int64")
        chunk = chunk.dropna(subset=["author_id", "name"])

        rows = [
            (
                int(_py(author_id)),
                str(_py(name)).strip(),
            )
            for author_id, name in chunk.itertuples(index=False, name=None)
        ]

        count = _bulk_insert_author(conn, schema, rows)
        conn.commit()
        print(f"  Chunk {chunk_index} committed | rows={count}")

    print(f"Finished {fp.name} in {time.perf_counter() - file_start:.2f}s")


def _import_book_author_csv(conn, schema: str, fp: Path) -> None:
    file_start = time.perf_counter()
    print(f"Importing {fp.name} ...")

    chunk_iter = pd.read_csv(fp, low_memory=False, chunksize=CHUNKSIZE)

    for chunk_index, chunk in enumerate(chunk_iter, start=1):
        if MAX_CHUNKS_PER_FILE and chunk_index > MAX_CHUNKS_PER_FILE:
            print(f"  Reached MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE} in {fp.name}")
            break

        _require_cols(chunk, ["book_isbn", "author_id"], fp.name)

        chunk["book_isbn"] = pd.to_numeric(chunk["book_isbn"], errors="coerce").astype("Int64")
        chunk["author_id"] = pd.to_numeric(chunk["author_id"], errors="coerce").astype("Int64")
        chunk = chunk.dropna(subset=["book_isbn", "author_id"]).drop_duplicates()

        rows = [
            (
                int(_py(book_isbn)),
                int(_py(author_id)),
            )
            for book_isbn, author_id in chunk.itertuples(index=False, name=None)
        ]

        count = _bulk_insert_book_author(conn, schema, rows)
        conn.commit()
        print(f"  Chunk {chunk_index} committed | rows={count}")

    print(f"Finished {fp.name} in {time.perf_counter() - file_start:.2f}s")


def _import_genre_csv(conn, schema: str, fp: Path) -> None:
    file_start = time.perf_counter()
    print(f"Importing {fp.name} ...")

    chunk_iter = pd.read_csv(fp, low_memory=False, chunksize=CHUNKSIZE)

    for chunk_index, chunk in enumerate(chunk_iter, start=1):
        if MAX_CHUNKS_PER_FILE and chunk_index > MAX_CHUNKS_PER_FILE:
            print(f"  Reached MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE} in {fp.name}")
            break

        _require_cols(chunk, ["genre_id", "name"], fp.name)

        chunk["genre_id"] = pd.to_numeric(chunk["genre_id"], errors="coerce").astype("Int64")
        chunk = chunk.dropna(subset=["genre_id", "name"])

        rows = [
            (
                int(_py(genre_id)),
                str(_py(name)).strip(),
            )
            for genre_id, name in chunk.itertuples(index=False, name=None)
        ]

        count = _bulk_insert_genre(conn, schema, rows)
        conn.commit()
        print(f"  Chunk {chunk_index} committed | rows={count}")

    print(f"Finished {fp.name} in {time.perf_counter() - file_start:.2f}s")


def _import_book_genre_csv(conn, schema: str, fp: Path) -> None:
    file_start = time.perf_counter()
    print(f"Importing {fp.name} ...")

    chunk_iter = pd.read_csv(fp, low_memory=False, chunksize=CHUNKSIZE)

    for chunk_index, chunk in enumerate(chunk_iter, start=1):
        if MAX_CHUNKS_PER_FILE and chunk_index > MAX_CHUNKS_PER_FILE:
            print(f"  Reached MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE} in {fp.name}")
            break

        _require_cols(chunk, ["book_isbn", "genre_id"], fp.name)

        chunk["book_isbn"] = pd.to_numeric(chunk["book_isbn"], errors="coerce").astype("Int64")
        chunk["genre_id"] = pd.to_numeric(chunk["genre_id"], errors="coerce").astype("Int64")
        chunk = chunk.dropna(subset=["book_isbn", "genre_id"]).drop_duplicates()

        rows = [
            (
                int(_py(book_isbn)),
                int(_py(genre_id)),
            )
            for book_isbn, genre_id in chunk.itertuples(index=False, name=None)
        ]

        count = _bulk_insert_book_genre(conn, schema, rows)
        conn.commit()
        print(f"  Chunk {chunk_index} committed | rows={count}")

    print(f"Finished {fp.name} in {time.perf_counter() - file_start:.2f}s")


def import_csvs_to_postgres(schema: str = SCHEMA, service: str = SERVICE) -> None:
    service = normalize_service_name(service)
    start_time = time.perf_counter()

    print(f"Connecting to PostgreSQL at {PGHOST}:{PGPORT}, database={PGDATABASE}, schema={schema}")
    print(f"Selected service: {service}")
    print(f"CSV directory: {CSV_DIR.resolve()}")
    print(f"Chunk size: {CHUNKSIZE}")
    print(f"Max chunks per file: {MAX_CHUNKS_PER_FILE if MAX_CHUNKS_PER_FILE > 0 else 'ALL'}")

    conn = _connect()
    conn.autocommit = False

    try:
        print(f"Ensuring schema/tables for service '{service}'")
        _execute_sql(conn, ddl_sql(schema, service))

        csv_files = _iter_service_csv_files(CSV_DIR, service)

        for fp in csv_files:
            filename = fp.name

            if filename == "book.csv":
                _import_book_csv(conn, schema, fp)
            elif filename == "rating.csv":
                _import_rating_csv(conn, schema, fp)
            elif filename == "author.csv":
                _import_author_csv(conn, schema, fp)
            elif filename == "book_author.csv":
                _import_book_author_csv(conn, schema, fp)
            elif filename == "genre.csv":
                _import_genre_csv(conn, schema, fp)
            elif filename == "book_genre.csv":
                _import_book_genre_csv(conn, schema, fp)
            else:
                raise ValueError(f"No importer defined for file: {filename}")

        elapsed = time.perf_counter() - start_time
        print(f"Import finished in {elapsed:.2f}s")

    finally:
        conn.close()
        print("PostgreSQL connection closed")


def drop_service_tables(schema: str = SCHEMA, service: str = SERVICE) -> None:
    service = normalize_service_name(service)
    conn = _connect()
    conn.autocommit = False

    try:
        sql = drop_tables_sql(schema, service)
        print(f"Dropping tables for service '{service}' in schema '{schema}'...")
        _execute_sql(conn, sql)
        print("Drop completed.")
    finally:
        conn.close()
        print("PostgreSQL connection closed")