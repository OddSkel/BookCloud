import os
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import List, Optional

import pandas as pd

# ----------------------
# Config
# ----------------------
FOLDER_PATH = Path("../dataset/csvs")
OUTPUT_DIR = Path("../dataset")
OUT_CSV = OUTPUT_DIR / "books_all.csv"

MAX_WORKERS = min(32, (os.cpu_count() or 8) * 2)
CHUNKSIZE = 250_000

# Deduplicação global
ID_COLUMN = "id"
KEEP = "first"  # "first" ou "last"


def _list_csvs(folder_path: Path) -> List[Path]:
    """Return sorted CSV files in the folder; raise if none are found."""
    files = sorted(folder_path.glob("*.csv"))
    if not files:
        raise FileNotFoundError(f"No CSV files found in: {folder_path.resolve()}")
    return files


def _read_one_csv(fp: Path) -> pd.DataFrame:
    """Read a single CSV file into a DataFrame."""
    return pd.read_csv(fp, low_memory=False)


def merge_csvs_to_single_csv(
    folder_path: Path = FOLDER_PATH,
    out_csv: Path = OUT_CSV,
) -> None:
    """Merge all CSV files in a folder into a single output CSV (streaming write)."""
    t0 = time.perf_counter()

    csv_files = _list_csvs(folder_path)
    out_csv.parent.mkdir(parents=True, exist_ok=True)

    # recria o ficheiro
    if out_csv.exists():
        out_csv.unlink()

    header_written = False
    total_rows_written = 0

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = {executor.submit(_read_one_csv, fp): fp for fp in csv_files}

        for i, fut in enumerate(as_completed(futures), 1):
            fp = futures[fut]
            try:
                df = fut.result()

                df.to_csv(
                    out_csv,
                    mode="a",
                    index=False,
                    header=not header_written,
                    chunksize=CHUNKSIZE,
                )
                header_written = True
                total_rows_written += len(df)

                print(f"[{i}/{len(csv_files)}] ✅ Appended: {fp.name} ({len(df):,} rows)")
            except Exception as e:
                print(f"[{i}/{len(csv_files)}] ❌ Failed: {fp.name} -> {e}")

    print(f"\n✅ Merge done (sem dedup ainda): {out_csv}")
    print(f"📤 Total rows appended: {total_rows_written:,}")
    print(f"⏱️ Merge time: {time.perf_counter() - t0:.3f}s")


def dedup_csv_globally_by_id(
    csv_path: Path = OUT_CSV,
    id_column: str = ID_COLUMN,
    keep: str = KEEP,
) -> None:
    """
    Remove duplicados GLOBALMENTE (considerando o ficheiro inteiro),
    com base no id
    """
    if keep not in {"first", "last"}:
        raise ValueError("keep must be 'first' or 'last'")

    if not csv_path.exists():
        raise FileNotFoundError(f"Merged CSV not found: {csv_path.resolve()}")

    t0 = time.perf_counter()

    df = pd.read_csv(csv_path, low_memory=False)

    if id_column not in df.columns:
        raise KeyError(
            f"Column '{id_column}' not found in merged file. Columns: {list(df.columns)}"
        )

    before = len(df)
    df = df.drop_duplicates(subset=id_column, keep=keep)  # ✅ GLOBAL
    after = len(df)

    # sobrescreve o ficheiro já deduplicado
    df.to_csv(csv_path, index=False, chunksize=CHUNKSIZE)

    print(f"\n🧹 Global dedup by '{id_column}' (keep='{keep}')")
    print(f"Before: {before:,} | After: {after:,} | Removed: {before - after:,}")
    print(f"⏱️ Dedup time: {time.perf_counter() - t0:.3f}s")


def inspect_merged_csv(
    csv_path: Path = OUT_CSV,
    n_head: int = 5,
    sep: str = ",",
    encoding: Optional[str] = None,
) -> pd.DataFrame:
    """Read the merged CSV and print quick info."""
    if not csv_path.exists():
        raise FileNotFoundError(f"Merged CSV not found: {csv_path.resolve()}")

    df = pd.read_csv(csv_path, sep=sep, encoding=encoding, low_memory=False)

    print("\n📌 HEAD:")
    print(df.head(n_head))

    print("\n📊 SHAPE:")
    print(f"Rows: {len(df):,}")
    print(f"Columns: {df.shape[1]:,}")

    print("\n🧾 COLUMNS:")
    for col in df.columns:
        print(f"- {col}")

    return df


if __name__ == "__main__":
    # 1) Faz o merge 
    merge_csvs_to_single_csv()

    # 2) Remove duplicados pelo id
    dedup_csv_globally_by_id(
        csv_path=OUT_CSV,
        id_column=ID_COLUMN,
        keep="first",
    )

    _ = inspect_merged_csv(OUT_CSV)