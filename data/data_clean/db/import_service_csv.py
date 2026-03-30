from __future__ import annotations

import argparse
import csv
import time
from pathlib import Path
from typing import Iterable, List, Sequence

import psycopg2
from psycopg2 import sql
from psycopg2.extras import execute_values


DEFAULT_CSV_DIR = Path("../normalized_out")
DEFAULT_SCHEMA = "public"
DEFAULT_CHUNK_SIZE = 10000


SERVICE_TABLES = {
    "book": [
        {
            "file": "book.csv",
            "table": "book",
            "columns": ["isbn", "name", "url", "summary_clean", "pub_year"],
            "ddl": """
                CREATE TABLE IF NOT EXISTS {schema}.book (
                    isbn BIGINT PRIMARY KEY,
                    name TEXT NOT NULL,
                    url TEXT,
                    summary_clean TEXT,
                    pub_year INTEGER
                );
            """,
        },
    ],
    "author": [
        {
            "file": "author.csv",
            "table": "author",
            "columns": ["author_id", "name"],
            "ddl": """
                CREATE TABLE IF NOT EXISTS {schema}.author (
                    author_id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL
                );
            """,
        },
        {
            "file": "book_author.csv",
            "table": "book_author",
            "columns": ["book_isbn", "author_id"],
            "ddl": """
                CREATE TABLE IF NOT EXISTS {schema}.book_author (
                    book_isbn BIGINT NOT NULL,
                    author_id INTEGER NOT NULL,
                    PRIMARY KEY (book_isbn, author_id)
                );
            """,
        },
    ],
    "genre": [
        {
            "file": "genre.csv",
            "table": "genre",
            "columns": ["genre_id", "name"],
            "ddl": """
                CREATE TABLE IF NOT EXISTS {schema}.genre (
                    genre_id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL
                );
            """,
        },
        {
            "file": "book_genre.csv",
            "table": "book_genre",
            "columns": ["book_isbn", "genre_id"],
            "ddl": """
                CREATE TABLE IF NOT EXISTS {schema}.book_genre (
                    book_isbn BIGINT NOT NULL,
                    genre_id INTEGER NOT NULL,
                    PRIMARY KEY (book_isbn, genre_id)
                );
            """,
        },
    ],
    "rating": [
        {
            "file": "rating.csv",
            "table": "rating",
            "columns": ["book_isbn", "star_rating", "num_ratings"],
            "ddl": """
                CREATE TABLE IF NOT EXISTS {schema}.rating (
                    book_isbn BIGINT PRIMARY KEY,
                    star_rating DOUBLE PRECISION,
                    num_ratings BIGINT
                );
            """,
        },
    ],
}


def normalize_service_name(service: str) -> str:
    value = service.strip().lower()
    aliases = {
        "books": "book",
        "authors": "author",
        "genres": "genre",
        "ratings": "rating",
    }
    return aliases.get(value, value)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Import normalized CSV files into PostgreSQL by service."
    )

    parser.add_argument(
        "--service",
        required=True,
        choices=["book", "author", "genre", "rating", "books", "authors", "genres", "ratings"],
        help="Service to import",
    )

    parser.add_argument(
        "--csv-dir",
        default=str(DEFAULT_CSV_DIR),
        help="Directory containing normalized CSV files",
    )

    parser.add_argument(
        "--host",
        default="localhost",
        help="PostgreSQL host",
    )

    parser.add_argument(
        "--port",
        type=int,
        default=5432,
        help="PostgreSQL port",
    )

    parser.add_argument(
        "--user",
        required=True,
        help="PostgreSQL user",
    )

    parser.add_argument(
        "--password",
        required=True,
        help="PostgreSQL password",
    )

    parser.add_argument(
        "--database",
        required=True,
        help="PostgreSQL database name",
    )

    parser.add_argument(
        "--schema",
        default=DEFAULT_SCHEMA,
        help="Target schema",
    )

    parser.add_argument(
        "--chunk-size",
        type=int,
        default=DEFAULT_CHUNK_SIZE,
        help="Number of rows per batch insert",
    )

    parser.add_argument(
        "--drop-before-import",
        action="store_true",
        help="Truncate target tables before importing",
    )

    parser.add_argument(
        "--create-tables",
        action="store_true",
        help="Create tables if they do not exist",
    )

    return parser.parse_args()


def get_connection(args):
    return psycopg2.connect(
        host=args.host,
        port=args.port,
        user=args.user,
        password=args.password,
        dbname=args.database,
    )


def ensure_schema(conn, schema: str) -> None:
    with conn.cursor() as cur:
        cur.execute(sql.SQL("CREATE SCHEMA IF NOT EXISTS {}").format(sql.Identifier(schema)))
    conn.commit()


def ensure_tables(conn, schema: str, service: str) -> None:
    with conn.cursor() as cur:
        for item in SERVICE_TABLES[service]:
            cur.execute(item["ddl"].format(schema=schema))
    conn.commit()


def truncate_tables(conn, schema: str, service: str) -> None:
    tables = [item["table"] for item in SERVICE_TABLES[service]]
    tables.reverse()

    with conn.cursor() as cur:
        for table in tables:
            print(f"Truncating {schema}.{table} ...")
            cur.execute(
                sql.SQL("TRUNCATE TABLE {}.{}").format(
                    sql.Identifier(schema),
                    sql.Identifier(table),
                )
            )
    conn.commit()


def chunked_rows(
    file_path: Path,
    expected_columns: Sequence[str],
    chunk_size: int,
) -> Iterable[List[List[object]]]:
    with file_path.open("r", encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)

        if reader.fieldnames is None:
            raise ValueError(f"{file_path.name}: CSV file has no header")

        missing = [col for col in expected_columns if col not in reader.fieldnames]
        if missing:
            raise ValueError(f"{file_path.name}: missing columns: {missing}")

        batch: List[List[object]] = []

        for row in reader:
            values = [normalize_value(row.get(col)) for col in expected_columns]
            batch.append(values)

            if len(batch) >= chunk_size:
                yield batch
                batch = []

        if batch:
            yield batch


def normalize_value(value: object) -> object:
    if value is None:
        return None

    text = str(value).strip()

    if text == "":
        return None

    lowered = text.lower()
    if lowered in {"nan", "none", "null", "<na>"}:
        return None

    return text


def import_csv_to_table(
    conn,
    schema: str,
    csv_dir: Path,
    file_name: str,
    table_name: str,
    columns: Sequence[str],
    chunk_size: int,
) -> int:
    file_path = csv_dir / file_name
    if not file_path.exists():
        raise FileNotFoundError(f"CSV file not found: {file_path}")

    full_table = f"{schema}.{table_name}"
    column_list = ", ".join(columns)

    insert_sql = f"""
        INSERT INTO {full_table} ({column_list})
        VALUES %s
        ON CONFLICT DO NOTHING
    """

    total_attempted = 0
    total_read = 0
    batch_number = 0

    print(f"Importing {file_name} -> {full_table}")

    with conn.cursor() as cur:
        for batch in chunked_rows(file_path, columns, chunk_size):
            batch_number += 1
            total_read += len(batch)

            execute_values(
                cur,
                insert_sql,
                batch,
                page_size=chunk_size,
            )

            total_attempted += len(batch)

            if batch_number % 10 == 0:
                conn.commit()
                print(
                    f"  batch={batch_number} | rows_read={total_read} | rows_attempted={total_attempted}"
                )

        conn.commit()

    print(f"Done: {file_name} | rows_read={total_read}")
    return total_attempted


def import_service(
    conn,
    schema: str,
    csv_dir: Path,
    service: str,
    chunk_size: int,
) -> None:
    total_files = 0
    total_rows = 0

    start = time.perf_counter()

    for item in SERVICE_TABLES[service]:
        attempted = import_csv_to_table(
            conn=conn,
            schema=schema,
            csv_dir=csv_dir,
            file_name=item["file"],
            table_name=item["table"],
            columns=item["columns"],
            chunk_size=chunk_size,
        )
        total_files += 1
        total_rows += attempted

    elapsed = time.perf_counter() - start

    print("")
    print("Import finished successfully.")
    print(f"Service: {service}")
    print(f"Files processed: {total_files}")
    print(f"Rows attempted: {total_rows}")
    print(f"Elapsed: {elapsed:.2f}s")


def main():
    args = parse_args()

    if args.chunk_size <= 0:
        raise ValueError("--chunk-size must be greater than 0")

    service = normalize_service_name(args.service)
    csv_dir = Path(args.csv_dir)

    if service not in SERVICE_TABLES:
        raise ValueError(f"Invalid service: {service}")

    if not csv_dir.exists() or not csv_dir.is_dir():
        raise FileNotFoundError(f"Invalid CSV directory: {csv_dir}")

    print("========================================")
    print("CSV IMPORT TO POSTGRESQL")
    print("========================================")
    print(f"Service: {service}")
    print(f"CSV dir: {csv_dir}")
    print(f"Host: {args.host}")
    print(f"Port: {args.port}")
    print(f"User: {args.user}")
    print(f"Database: {args.database}")
    print(f"Schema: {args.schema}")
    print(f"Chunk size: {args.chunk_size}")
    print(f"Create tables: {'yes' if args.create_tables else 'no'}")
    print(f"Drop before import: {'yes' if args.drop_before_import else 'no'}")
    print("========================================")

    conn = get_connection(args)

    try:
        ensure_schema(conn, args.schema)

        if args.create_tables:
            ensure_tables(conn, args.schema, service)

        if args.drop_before_import:
            truncate_tables(conn, args.schema, service)

        import_service(
            conn=conn,
            schema=args.schema,
            csv_dir=csv_dir,
            service=service,
            chunk_size=args.chunk_size,
        )
    finally:
        conn.close()


if __name__ == "__main__":
    main()
