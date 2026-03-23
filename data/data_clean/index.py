import argparse
import os
import time

import psycopg2

from create_postgresql_schema import (
    PGDATABASE,
    PGHOST,
    PGPASSWORD,
    PGPORT,
    PGUSER,
    SCHEMA,
    CHUNKSIZE,
    MAX_CHUNKS_PER_FILE,
    CSV_DIR,
    ddl_sql,
    drop_service_tables,
    import_csvs_to_postgres,
    normalize_service_name,
)


MAX_WAIT_SECONDS = int(os.getenv("PG_WAIT_SECONDS", os.getenv("POSTGRES_WAIT_SECONDS", "90")))


def wait_for_postgres():
    print(f"Waiting for PostgreSQL at {PGHOST}:{PGPORT} (db={PGDATABASE})...")
    t0 = time.time()
    last_err = None

    while True:
        try:
            conn = psycopg2.connect(
                host=PGHOST,
                port=PGPORT,
                user=PGUSER,
                password=PGPASSWORD,
                dbname=PGDATABASE,
            )
            conn.close()
            print("PostgreSQL is ready.")
            return
        except Exception as e:
            last_err = e
            if time.time() - t0 > MAX_WAIT_SECONDS:
                raise RuntimeError(
                    f"PostgreSQL was not ready within {MAX_WAIT_SECONDS}s. Last error: {last_err}"
                )
            time.sleep(2)


def ensure_schema(schema: str, service: str):
    print(f"Ensuring schema and tables ({schema}) for service '{service}'...")
    conn = psycopg2.connect(
        host=PGHOST,
        port=PGPORT,
        user=PGUSER,
        password=PGPASSWORD,
        dbname=PGDATABASE,
    )
    try:
        conn.autocommit = True
        with conn.cursor() as cur:
            cur.execute(ddl_sql(schema, service))
        print("Schema and tables are ready.")
    finally:
        conn.close()


def print_execution_plan(service: str, drop_all: bool):
    print("\nExecution plan:")
    print(f"   POSTGRES_HOST={PGHOST}")
    print(f"   POSTGRES_PORT={PGPORT}")
    print(f"   POSTGRES_USER={PGUSER}")
    print(f"   POSTGRES_DB={PGDATABASE}")
    print(f"   PGSCHEMA={SCHEMA}")
    print(f"   CSV_DIR={CSV_DIR}")
    print(f"   SERVICE={service}")
    print(f"   CHUNKSIZE={CHUNKSIZE}")
    print(f"   MAX_CHUNKS_PER_FILE={MAX_CHUNKS_PER_FILE}")
    print(f"   DROP_ALL={'yes' if drop_all else 'no'}\n")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Import normalized CSVs into PostgreSQL by microservice."
    )

    parser.add_argument(
        "-S",
        "--service",
        required=False,
        default=os.getenv("SERVICE", "all"),
        help="Service to import: bookCatalog, ratingCatalog, authorCatalog, genreAnalysis, all",
    )

    parser.add_argument(
        "--drop-all",
        action="store_true",
        help="Drop all tables for the selected service in the current database/schema and exit.",
    )

    return parser.parse_args()


def main():
    args = parse_args()
    service = normalize_service_name(args.service)

    print_execution_plan(service=service, drop_all=args.drop_all)
    wait_for_postgres()

    if args.drop_all:
        drop_service_tables(schema=SCHEMA, service=service)
        return

    ensure_schema(schema=SCHEMA, service=service)
    import_csvs_to_postgres(schema=SCHEMA, service=service)


if __name__ == "__main__":
    main()