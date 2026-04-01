from __future__ import annotations

import argparse
import csv
import re
import shutil
import time
from pathlib import Path
from typing import Dict, List, Optional

import pandas as pd


DEFAULT_INPUT = Path("../../dataset/compiled_books.csv")
DEFAULT_OUTPUT = Path("../normalized_out")
DEFAULT_CHUNKSIZE = 100000
DEFAULT_MAX_ROWS = 0


def _clean_text(value) -> str:
    if value is None:
        return ""
    text = str(value).strip()
    text = re.sub(r"\s+", " ", text)
    return text


def _normalize_isbn(value) -> Optional[str]:
    if value is None:
        return None

    try:
        if pd.isna(value):
            return None
    except Exception:
        pass

    text = re.sub(r"[^0-9]", "", str(value)).strip()
    if not text:
        return None

    if not text.isdigit():
        return None

    return text


def _isbn_to_int(value) -> Optional[int]:
    isbn = _normalize_isbn(value)
    if not isbn:
        return None
    try:
        return int(isbn)
    except ValueError:
        return None


def _parse_multi_value_field(value) -> List[str]:
    if value is None:
        return []

    try:
        if pd.isna(value):
            return []
    except Exception:
        pass

    text = _clean_text(value)
    if not text:
        return []

    items: List[str] = []

    if text.startswith("[") and text.endswith("]"):
        quoted = re.findall(r"'([^']+)'|\"([^\"]+)\"", text)
        if quoted:
            for a, b in quoted:
                item = _clean_text(a or b)
                if item:
                    items.append(item)
        else:
            inner = text[1:-1].strip()
            if inner:
                parts = re.split(r"\s*[|;,/]\s*|\s{2,}|\n+", inner)
                for part in parts:
                    item = _clean_text(part)
                    if item:
                        items.append(item)
    else:
        parts = re.split(r"\s*[|;,/]\s*|\s{2,}|\n+", text)
        for part in parts:
            item = _clean_text(part)
            if item:
                items.append(item)

    seen = set()
    out: List[str] = []
    for item in items:
        key = item.casefold()
        if key not in seen:
            seen.add(key)
            out.append(item)

    return out


def _safe_float(value) -> Optional[float]:
    if value is None:
        return None
    try:
        if pd.isna(value):
            return None
    except Exception:
        pass
    try:
        return float(value)
    except Exception:
        return None


def _safe_int(value) -> Optional[int]:
    if value is None:
        return None
    try:
        if pd.isna(value):
            return None
    except Exception:
        pass
    try:
        return int(float(value))
    except Exception:
        return None


def _require_cols(df: pd.DataFrame, cols: List[str], filename: str) -> None:
    missing = [c for c in cols if c not in df.columns]
    if missing:
        raise ValueError(f"[{filename}] missing columns: {missing}")


def _prepare_output_dir(output_dir: Path) -> None:
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)


def _deduplicate_csv(
    input_path: Path,
    output_path: Path,
    subset: List[str],
    sort_by: Optional[List[str]] = None,
    keep: str = "last",
) -> None:
    df = pd.read_csv(input_path, low_memory=False)

    if sort_by:
        df = df.sort_values(sort_by)

    df = df.drop_duplicates(subset=subset, keep=keep)
    df.to_csv(output_path, index=False)


def _deduplicate_pairs_csv(
    input_path: Path,
    output_path: Path,
    subset: List[str],
) -> None:
    df = pd.read_csv(input_path, low_memory=False)
    df = df.drop_duplicates(subset=subset, keep="first")
    df.to_csv(output_path, index=False)


def build_normalized_outputs(
    input_file: Path,
    output_dir: Path,
    chunksize: int,
    max_rows: int,
) -> None:
    if not input_file.exists():
        raise FileNotFoundError(f"Input file not found: {input_file.resolve()}")

    start = time.perf_counter()
    _prepare_output_dir(output_dir)

    tmp_dir = output_dir / "_tmp"
    tmp_dir.mkdir(parents=True, exist_ok=True)

    tmp_book_fp = tmp_dir / "book_raw.csv"
    tmp_rating_fp = tmp_dir / "rating_raw.csv"
    tmp_book_author_fp = tmp_dir / "book_author_raw.csv"
    tmp_book_genre_fp = tmp_dir / "book_genre_raw.csv"

    book_fp = output_dir / "book.csv"
    rating_fp = output_dir / "rating.csv"
    author_fp = output_dir / "author.csv"
    book_author_fp = output_dir / "book_author.csv"
    genre_fp = output_dir / "genre.csv"
    book_genre_fp = output_dir / "book_genre.csv"

    author_name_to_id: Dict[str, int] = {}
    genre_name_to_id: Dict[str, int] = {}

    next_author_id = 1
    next_genre_id = 1
    total_rows_read = 0
    total_valid_rows = 0
    chunk_count = 0

    print(f"Input file: {input_file.resolve()}")
    print(f"Output dir: {output_dir.resolve()}")
    print(f"Chunk size: {chunksize}")
    print(f"Max rows: {'ALL' if max_rows <= 0 else max_rows}")

    with (
        tmp_book_fp.open("w", newline="", encoding="utf-8") as book_f,
        tmp_rating_fp.open("w", newline="", encoding="utf-8") as rating_f,
        tmp_book_author_fp.open("w", newline="", encoding="utf-8") as book_author_f,
        tmp_book_genre_fp.open("w", newline="", encoding="utf-8") as book_genre_f,
    ):
        book_writer = csv.writer(book_f)
        rating_writer = csv.writer(rating_f)
        book_author_writer = csv.writer(book_author_f)
        book_genre_writer = csv.writer(book_genre_f)

        book_writer.writerow(["isbn", "name", "url", "summary_clean", "pub_year"])
        rating_writer.writerow(["book_isbn", "star_rating", "num_ratings"])
        book_author_writer.writerow(["book_isbn", "author_id"])
        book_genre_writer.writerow(["book_isbn", "genre_id"])

        chunk_iter = pd.read_csv(
            input_file,
            low_memory=False,
            chunksize=chunksize,
            dtype={"isbn": "string"},
        )

        for chunk in chunk_iter:
            if max_rows > 0 and total_rows_read >= max_rows:
                break

            chunk_count += 1
            original_len = len(chunk)

            if max_rows > 0:
                remaining = max_rows - total_rows_read
                if remaining <= 0:
                    break
                if original_len > remaining:
                    chunk = chunk.iloc[:remaining].copy()

            total_rows_read += len(chunk)

            _require_cols(
                chunk,
                [
                    "name",
                    "author",
                    "url",
                    "genres",
                    "summary_clean",
                    "pub_year",
                    "star_rating",
                    "num_ratings",
                    "isbn",
                ],
                input_file.name,
            )

            print(f"Processing chunk {chunk_count} | rows={len(chunk)}")

            for row in chunk.itertuples(index=False):
                isbn = _isbn_to_int(getattr(row, "isbn", None))
                if isbn is None:
                    continue

                name = _clean_text(getattr(row, "name", ""))
                if not name:
                    continue

                total_valid_rows += 1

                url = _clean_text(getattr(row, "url", ""))
                summary_clean = _clean_text(getattr(row, "summary_clean", ""))
                pub_year = _safe_int(getattr(row, "pub_year", None))
                star_rating = _safe_float(getattr(row, "star_rating", None))
                num_ratings = _safe_int(getattr(row, "num_ratings", None))

                book_writer.writerow([isbn, name, url, summary_clean, pub_year])
                rating_writer.writerow([isbn, star_rating, num_ratings])

                author_names = _parse_multi_value_field(getattr(row, "author", None))
                for author_name in author_names:
                    author_id = author_name_to_id.get(author_name)
                    if author_id is None:
                        author_id = next_author_id
                        author_name_to_id[author_name] = author_id
                        next_author_id += 1
                    book_author_writer.writerow([isbn, author_id])

                genre_names = _parse_multi_value_field(getattr(row, "genres", None))
                for genre_name in genre_names:
                    genre_id = genre_name_to_id.get(genre_name)
                    if genre_id is None:
                        genre_id = next_genre_id
                        genre_name_to_id[genre_name] = genre_id
                        next_genre_id += 1
                    book_genre_writer.writerow([isbn, genre_id])

            print(
                f"Chunk {chunk_count} done | "
                f"total_rows_read={total_rows_read} | "
                f"valid_isbn_rows={total_valid_rows} | "
                f"authors_so_far={len(author_name_to_id)} | "
                f"genres_so_far={len(genre_name_to_id)}"
            )

    print("Deduplicating book.csv ...")
    _deduplicate_csv(
        input_path=tmp_book_fp,
        output_path=book_fp,
        subset=["isbn"],
        sort_by=["isbn"],
        keep="last",
    )

    print("Deduplicating rating.csv ...")
    _deduplicate_csv(
        input_path=tmp_rating_fp,
        output_path=rating_fp,
        subset=["book_isbn"],
        sort_by=["book_isbn"],
        keep="last",
    )

    print("Deduplicating book_author.csv ...")
    _deduplicate_pairs_csv(
        input_path=tmp_book_author_fp,
        output_path=book_author_fp,
        subset=["book_isbn", "author_id"],
    )

    print("Deduplicating book_genre.csv ...")
    _deduplicate_pairs_csv(
        input_path=tmp_book_genre_fp,
        output_path=book_genre_fp,
        subset=["book_isbn", "genre_id"],
    )

    with author_fp.open("w", newline="", encoding="utf-8") as author_f:
        author_writer = csv.writer(author_f)
        author_writer.writerow(["author_id", "name"])
        for name, author_id in sorted(author_name_to_id.items(), key=lambda x: x[1]):
            author_writer.writerow([author_id, name])

    with genre_fp.open("w", newline="", encoding="utf-8") as genre_f:
        genre_writer = csv.writer(genre_f)
        genre_writer.writerow(["genre_id", "name"])
        for name, genre_id in sorted(genre_name_to_id.items(), key=lambda x: x[1]):
            genre_writer.writerow([genre_id, name])

    shutil.rmtree(tmp_dir, ignore_errors=True)

    elapsed = time.perf_counter() - start
    print("")
    print("Normalized output created successfully.")
    print(f"Files written to: {output_dir.resolve()}")
    print(f"Rows read: {total_rows_read}")
    print(f"Valid ISBN rows: {total_valid_rows}")
    print(f"Unique authors: {len(author_name_to_id)}")
    print(f"Unique genres: {len(genre_name_to_id)}")
    print(f"Elapsed: {elapsed:.2f}s")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Create normalized CSV outputs from compiled_books.csv"
    )
    parser.add_argument(
        "--input",
        default=str(DEFAULT_INPUT),
        help="Path to source compiled CSV file",
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="Output directory for normalized CSVs",
    )
    parser.add_argument(
        "--chunksize",
        type=int,
        default=DEFAULT_CHUNKSIZE,
        help="Number of rows per chunk",
    )
    parser.add_argument(
        "--max-rows",
        type=int,
        default=DEFAULT_MAX_ROWS,
        help="Maximum number of source rows to read (0 = all)",
    )
    return parser.parse_args()


def main():
    args = parse_args()

    input_file = Path(args.input)
    output_dir = Path(args.output)

    if args.chunksize <= 0:
        raise ValueError("--chunksize must be greater than 0")

    if args.max_rows < 0:
        raise ValueError("--max-rows must be >= 0")

    build_normalized_outputs(
        input_file=input_file,
        output_dir=output_dir,
        chunksize=args.chunksize,
        max_rows=args.max_rows,
    )


if __name__ == "__main__":
    main()
