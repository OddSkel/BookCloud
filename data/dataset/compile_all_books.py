from __future__ import annotations

import os
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from threading import Lock
from typing import List
import re

import pandas as pd


INPUT_DIR = Path("csvs")
OUTPUT_FILE = Path("compiled_books.csv")
CHUNK_SIZE = 50_000
MAX_WORKERS = min(8, (os.cpu_count() or 4))


# Global structures for deduplication across all files/chunks
seen_isbns = set()
seen_lock = Lock()


def normalize_isbn(value) -> str | None:
    if value is None:
        return None

    try:
        if pd.isna(value):
            return None
    except Exception:
        pass

    text = str(value).strip()
    if not text or text.lower() == "nan":
        return None

    if re.fullmatch(r"\d+\.0+", text):
        text = text.split(".", 1)[0]

    digits = re.sub(r"[^0-9]", "", text)
    if not digits:
        return None

    return digits


def process_chunk(chunk: pd.DataFrame) -> pd.DataFrame:
    """
    Rename/remove columns and perform global deduplication by isbn.
    """
    # Rename isbn_clean -> isbn if it exists
    if "isbn_clean" in chunk.columns:
        chunk = chunk.rename(columns={"isbn_clean": "isbn"})

    # Remove id column if it exists
    if "id" in chunk.columns:
        chunk = chunk.drop(columns=["id"])

    # If isbn column does not exist, return empty DataFrame
    if "isbn" not in chunk.columns:
        return pd.DataFrame(columns=chunk.columns)

    # Normalize isbn values to avoid false duplicates
    chunk["isbn"] = chunk["isbn"].map(normalize_isbn)

    # Remove null/empty values
    chunk = chunk.loc[chunk["isbn"].notna()].copy()

    # Deduplicate within the chunk first
    chunk = chunk.drop_duplicates(subset=["isbn"], keep="first")

    # Global deduplication across all files
    kept_rows = []

    with seen_lock:
        for _, row in chunk.iterrows():
            isbn = row["isbn"]
            if isbn not in seen_isbns:
                seen_isbns.add(isbn)
                kept_rows.append(row)

    if not kept_rows:
        return pd.DataFrame(columns=chunk.columns)

    return pd.DataFrame(kept_rows, columns=chunk.columns)


def process_file(file_path: Path) -> List[pd.DataFrame]:
    """
    Read a CSV file in chunks, process each chunk and return valid chunks.
    """
    print(f"[INFO] Processing: {file_path.name}")
    processed_chunks = []

    try:
        for chunk in pd.read_csv(file_path, chunksize=CHUNK_SIZE, low_memory=False):
            cleaned_chunk = process_chunk(chunk)

            if not cleaned_chunk.empty:
                processed_chunks.append(cleaned_chunk)

        print(f"[OK] Finished: {file_path.name}")
    except Exception as e:
        print(f"[ERROR] Failed to process {file_path.name}: {e}")

    return processed_chunks


def main() -> None:
    # Validate input directory
    if not INPUT_DIR.exists() or not INPUT_DIR.is_dir():
        raise FileNotFoundError(f"Folder '{INPUT_DIR}' does not exist.")

    csv_files = sorted(INPUT_DIR.glob("*.csv"))

    if not csv_files:
        raise FileNotFoundError(f"No CSV files found in '{INPUT_DIR}'.")

    # Remove previous output file if it exists
    if OUTPUT_FILE.exists():
        OUTPUT_FILE.unlink()

    all_processed_chunks: List[pd.DataFrame] = []

    # Parallel processing using threads
    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = {
            executor.submit(process_file, file_path): file_path
            for file_path in csv_files
        }

        for future in as_completed(futures):
            file_path = futures[future]
            try:
                result_chunks = future.result()
                all_processed_chunks.extend(result_chunks)
            except Exception as e:
                print(f"[ERROR] Thread failed for {file_path.name}: {e}")

    if not all_processed_chunks:
        print("[WARNING] No valid data found to write.")
        return

    print("[INFO] Merging all data into final file...")

    final_df = pd.concat(all_processed_chunks, ignore_index=True)

    # Final safety deduplication by isbn
    if "isbn" in final_df.columns:
        final_df = final_df.drop_duplicates(subset=["isbn"], keep="first")

    final_df.to_csv(OUTPUT_FILE, index=False, encoding="utf-8")

    print(f"[SUCCESS] Final file created: {OUTPUT_FILE}")
    print(f"[INFO] Total records: {len(final_df)}")


if __name__ == "__main__":
    main()
