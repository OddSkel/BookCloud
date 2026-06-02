#!/usr/bin/env python3
from __future__ import annotations

import csv
import os
import shutil
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
SOURCE_DIR = REPO_ROOT / "data" / "data_clean" / "normalized_out"
OUTPUT_DIR = REPO_ROOT / "data" / "data_clean" / "normalized_sample"
DEFAULT_SAMPLE_SIZE = 500_000

EXPECTED_HEADERS = {
    "book.csv": ["isbn", "name", "url", "summary_clean", "pub_year"],
    "rating.csv": ["book_isbn", "star_rating", "num_ratings"],
    "author.csv": ["author_id", "name"],
    "book_author.csv": ["book_isbn", "author_id"],
    "genre.csv": ["genre_id", "name"],
    "book_genre.csv": ["book_isbn", "genre_id"],
}


def sample_size_from_env() -> int:
    raw_value = os.getenv("BOOKCLOUD_SAMPLE_SIZE", str(DEFAULT_SAMPLE_SIZE))

    try:
        sample_size = int(raw_value)
    except ValueError as exc:
        raise SystemExit(
            f"BOOKCLOUD_SAMPLE_SIZE must be an integer, got: {raw_value!r}"
        ) from exc

    if sample_size < 1:
        raise SystemExit("BOOKCLOUD_SAMPLE_SIZE must be greater than zero")

    return sample_size


def ensure_source_files() -> None:
    if not SOURCE_DIR.exists():
        raise SystemExit(f"Source directory not found: {SOURCE_DIR}")

    missing_files = [
        filename for filename in EXPECTED_HEADERS if not (SOURCE_DIR / filename).exists()
    ]
    if missing_files:
        missing = ", ".join(sorted(missing_files))
        raise SystemExit(f"Missing normalized CSV files in {SOURCE_DIR}: {missing}")


def prepare_output_dir() -> None:
    if OUTPUT_DIR.exists():
        shutil.rmtree(OUTPUT_DIR)
    OUTPUT_DIR.mkdir(parents=True)


def validate_headers(reader: csv.DictReader, file_path: Path) -> None:
    expected = EXPECTED_HEADERS[file_path.name]
    if reader.fieldnames is None:
        raise SystemExit(f"{file_path}: CSV file has no header")

    missing_columns = [column for column in expected if column not in reader.fieldnames]
    if missing_columns:
        missing = ", ".join(missing_columns)
        raise SystemExit(f"{file_path}: missing expected columns: {missing}")


def read_rows(filename: str) -> list[dict[str, str]]:
    file_path = SOURCE_DIR / filename
    with file_path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        validate_headers(reader, file_path)
        return [
            {column: row.get(column, "") for column in EXPECTED_HEADERS[filename]}
            for row in reader
        ]


def read_limited_rows(filename: str, limit: int) -> list[dict[str, str]]:
    rows = []
    file_path = SOURCE_DIR / filename

    with file_path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        validate_headers(reader, file_path)

        for row in reader:
            rows.append(
                {column: row.get(column, "") for column in EXPECTED_HEADERS[filename]}
            )
            if len(rows) >= limit:
                break

    return rows


def write_rows(filename: str, rows: Iterable[dict[str, str]]) -> int:
    count = 0
    file_path = OUTPUT_DIR / filename
    fieldnames = EXPECTED_HEADERS[filename]

    with file_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames, extrasaction="ignore")
        writer.writeheader()

        for row in rows:
            writer.writerow({column: row.get(column, "") for column in fieldnames})
            count += 1

    return count


def main() -> None:
    sample_size = sample_size_from_env()

    ensure_source_files()
    prepare_output_dir()

    selected_books = read_limited_rows("book.csv", sample_size)
    selected_isbns = {row["isbn"] for row in selected_books}

    ratings = [
        row for row in read_rows("rating.csv") if row["book_isbn"] in selected_isbns
    ]

    book_authors = [
        row for row in read_rows("book_author.csv") if row["book_isbn"] in selected_isbns
    ]
    selected_author_ids = {row["author_id"] for row in book_authors}
    authors = [
        row for row in read_rows("author.csv") if row["author_id"] in selected_author_ids
    ]

    book_genres = [
        row for row in read_rows("book_genre.csv") if row["book_isbn"] in selected_isbns
    ]
    selected_genre_ids = {row["genre_id"] for row in book_genres}
    genres = [
        row for row in read_rows("genre.csv") if row["genre_id"] in selected_genre_ids
    ]

    counts = {
        "books": write_rows("book.csv", selected_books),
        "ratings": write_rows("rating.csv", ratings),
        "authors": write_rows("author.csv", authors),
        "book_author_relations": write_rows("book_author.csv", book_authors),
        "genres": write_rows("genre.csv", genres),
        "book_genre_relations": write_rows("book_genre.csv", book_genres),
    }

    print("Dataset sample created.")
    print(f"Books: {counts['books']}")
    print(f"Ratings: {counts['ratings']}")
    print(f"Authors: {counts['authors']}")
    print(f"Book-author relations: {counts['book_author_relations']}")
    print(f"Genres: {counts['genres']}")
    print(f"Book-genre relations: {counts['book_genre_relations']}")
    print(f"Output directory: {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
