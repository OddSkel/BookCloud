import os
import time
import psycopg2

from create_postgresql_schema import import_csvs_to_postgres, ddl_sql


PGHOST = os.getenv("PGHOST", "localhost")
PGPORT = int(os.getenv("PGPORT", "5432"))
PGUSER = os.getenv("PGUSER", "bookcloud")
PGPASSWORD = os.getenv("PGPASSWORD", "bookcloud")
PGDATABASE = os.getenv("PGDATABASE", "bookcloud_db")
SCHEMA = os.getenv("PGSCHEMA", "bookcloud")

MAX_WAIT_SECONDS = int(os.getenv("PG_WAIT_SECONDS", "90"))


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


def ensure_schema():
    print(f"Ensuring schema and tables ({SCHEMA})...")
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
            cur.execute(ddl_sql(SCHEMA))
        print("Schema and tables are ready.")
    finally:
        conn.close()


def print_test_plan():
    print("\nExecution plan (env):")
    print(f"   CSV_DIR={os.getenv('CSV_DIR', '../dataset/csvs')}")
    print(f"   CSV_ONLY={os.getenv('CSV_ONLY', '') or '(none)'}")
    print(f"   CSV_SKIP={os.getenv('CSV_SKIP', '') or '(none)'}")
    print(f"   CSV_LIMIT={os.getenv('CSV_LIMIT', '0')}")
    print(f"   MAX_CHUNKS_PER_FILE={os.getenv('MAX_CHUNKS_PER_FILE', '0')}")
    print(f"   CHUNKSIZE={os.getenv('CHUNKSIZE', '200000')}")
    print(f"   BOOK_KEEP={os.getenv('BOOK_KEEP', 'first')}\n")


def main():
    print_test_plan()
    wait_for_postgres()
    ensure_schema()
    import_csvs_to_postgres(schema=SCHEMA)


if __name__ == "__main__":
    main()
