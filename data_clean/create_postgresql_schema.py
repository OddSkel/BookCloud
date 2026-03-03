from pathlib import Path
from typing import Optional, Tuple
import re

import pandas as pd

FILE = "../dataset/csvs/action.csv"
CSV_DIR = Path("../dataset/csvs")
OUT_DIR = Path("./normalized_out")


# ----------------------------
# Helpers de normalização
# ----------------------------
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
    """
    Retorna string com apenas dígitos e X (ISBN-10 pode ter X).
    """
    if isbn is None or (isinstance(isbn, float) and pd.isna(isbn)):
        return None
    s = re.sub(r"[^0-9Xx]", "", str(isbn)).upper().strip()
    return s if s else None


def _isbn_to_int(isbn_clean) -> Optional[int]:
    """
    Converte para int. Se tiver NaN/None/float, trata.
    Se tiver 'X' (ISBN-10), retorna None para manter BIGINT consistente.
    """
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


# ----------------------------
# Pipeline principal
# ----------------------------
def load_df(csv_path: Path, sep: str = ",", encoding: Optional[str] = None) -> pd.DataFrame:
    if not csv_path.exists():
        raise FileNotFoundError(f"CSV not found: {csv_path.resolve()}")
    return pd.read_csv(csv_path, sep=sep, encoding=encoding, low_memory=False)


# 🔥 BOOK (ISBN BIGINT, sem id)
def build_book(df: pd.DataFrame) -> pd.DataFrame:
    cols = ["name", "url", "summary_clean", "pub_year", "isbn_clean"]
    missing = [c for c in cols if c not in df.columns]
    if missing:
        raise ValueError(f"Missing columns for book: {missing}")

    book = df[cols].copy()

    # normaliza e converte ISBN para inteiro (pode virar None)
    book["isbn_clean"] = book["isbn_clean"].map(_normalize_isbn).map(_isbn_to_int)
    book["isbn_clean"] = pd.to_numeric(book["isbn_clean"], errors="coerce").astype("Int64")

    # remove linhas sem ISBN (PK)
    book = book.dropna(subset=["isbn_clean"]).copy()

    book["name"] = book["name"].map(_clean_text)
    book["url"] = book["url"].map(_clean_text)
    book["summary_clean"] = book["summary_clean"].astype(str).map(_clean_text)
    book["pub_year"] = pd.to_numeric(book["pub_year"], errors="coerce").astype("Int64")

    print(f"\n📚 BEFORE drop_duplicates (book): {len(book):,} rows")

    # dedup por ISBN
    book = book.drop_duplicates(subset=["isbn_clean"])

    print(f"📚 AFTER drop_duplicates (book): {len(book):,} rows")

    # ISBN primeiro + rename para isbn
    book = book[["isbn_clean", "name", "url", "summary_clean", "pub_year"]].rename(
        columns={"isbn_clean": "isbn"}
    )

    return book


def build_rating(df: pd.DataFrame, book_df: pd.DataFrame) -> pd.DataFrame:
    needed = ["isbn_clean", "star_rating", "num_ratings"]
    missing = [c for c in needed if c not in df.columns]
    if missing:
        raise ValueError(f"Missing columns for rating: {missing}")

    rating = df[needed].copy()

    rating["isbn_clean"] = rating["isbn_clean"].map(_normalize_isbn).map(_isbn_to_int)
    rating["isbn_clean"] = pd.to_numeric(rating["isbn_clean"], errors="coerce").astype("Int64")
    rating = rating.dropna(subset=["isbn_clean"]).copy()

    rating["star_rating"] = pd.to_numeric(rating["star_rating"], errors="coerce")
    rating["num_ratings"] = pd.to_numeric(rating["num_ratings"], errors="coerce").astype("Int64")

    # agora book_df tem coluna "isbn"
    rating = rating[rating["isbn_clean"].isin(book_df["isbn"])].copy()

    rating = (
        rating.sort_values(["isbn_clean"])
        .groupby("isbn_clean", as_index=False)
        .agg(
            star_rating=("star_rating", "first"),
            num_ratings=("num_ratings", "first"),
        )
    )

    rating = rating.rename(columns={"isbn_clean": "book_isbn"})
    return rating


def build_author_and_link(df: pd.DataFrame, book_df: pd.DataFrame) -> Tuple[pd.DataFrame, pd.DataFrame]:
    if "author" not in df.columns:
        raise ValueError("Missing column: author")
    if "isbn_clean" not in df.columns:
        raise ValueError("Missing column: isbn_clean")

    tmp = df[["isbn_clean", "author"]].copy()
    tmp["isbn_clean"] = tmp["isbn_clean"].map(_normalize_isbn).map(_isbn_to_int)
    tmp["isbn_clean"] = pd.to_numeric(tmp["isbn_clean"], errors="coerce").astype("Int64")
    tmp = tmp.dropna(subset=["isbn_clean"]).copy()

    tmp["author_list"] = tmp["author"].apply(_parse_list_string)
    tmp = tmp.explode("author_list")

    tmp["author_list"] = tmp["author_list"].map(_clean_text)
    tmp = tmp[tmp["author_list"].notna() & (tmp["author_list"] != "")].copy()

    author = (
        tmp[["author_list"]]
        .drop_duplicates()
        .rename(columns={"author_list": "name"})
        .sort_values("name")
        .reset_index(drop=True)
    )
    author["author_id"] = range(1, len(author) + 1)

    book_author = tmp.merge(author, left_on="author_list", right_on="name", how="inner")
    book_author = book_author[["isbn_clean", "author_id"]].rename(columns={"isbn_clean": "book_isbn"}).drop_duplicates()

    # book_df agora tem isbn
    book_author = book_author[book_author["book_isbn"].isin(book_df["isbn"])].copy()

    return author[["author_id", "name"]], book_author


def build_genre_and_link(df: pd.DataFrame, book_df: pd.DataFrame) -> Tuple[pd.DataFrame, pd.DataFrame]:
    if "genres" not in df.columns:
        raise ValueError("Missing column: genres")
    if "isbn_clean" not in df.columns:
        raise ValueError("Missing column: isbn_clean")

    tmp = df[["isbn_clean", "genres"]].copy()
    tmp["isbn_clean"] = tmp["isbn_clean"].map(_normalize_isbn).map(_isbn_to_int)
    tmp["isbn_clean"] = pd.to_numeric(tmp["isbn_clean"], errors="coerce").astype("Int64")
    tmp = tmp.dropna(subset=["isbn_clean"]).copy()

    tmp["genre_list"] = tmp["genres"].apply(_parse_list_string)
    tmp = tmp.explode("genre_list")

    tmp["genre_list"] = tmp["genre_list"].map(_clean_text)
    tmp = tmp[tmp["genre_list"].notna() & (tmp["genre_list"] != "")].copy()

    genre = (
        tmp[["genre_list"]]
        .drop_duplicates()
        .rename(columns={"genre_list": "name"})
        .sort_values("name")
        .reset_index(drop=True)
    )
    genre["genre_id"] = range(1, len(genre) + 1)

    book_genre = tmp.merge(genre, left_on="genre_list", right_on="name", how="inner")
    book_genre = book_genre[["isbn_clean", "genre_id"]].rename(columns={"isbn_clean": "book_isbn"}).drop_duplicates()

    # book_df agora tem isbn
    book_genre = book_genre[book_genre["book_isbn"].isin(book_df["isbn"])].copy()

    return genre[["genre_id", "name"]], book_genre


def generate_postgres_ddl(schema: str = "bookcloud") -> str:
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
""".strip()


def load_multiple_csvs(folder: Path, limit: int = 15) -> pd.DataFrame:
    if not folder.exists():
        raise FileNotFoundError(f"Folder not found: {folder.resolve()}")

    csv_files = sorted(folder.glob("*.csv"))[:limit]

    if not csv_files:
        raise ValueError("No CSV files found in folder.")

    print(f"\n📂 Found {len(csv_files)} CSV files:")
    for f in csv_files:
        print(" -", f.name)

    dfs = []
    for file in csv_files:
        df = pd.read_csv(file, low_memory=False)
        dfs.append(df)

    combined_df = pd.concat(dfs, ignore_index=True)

    print(f"\n📊 Combined shape: {combined_df.shape[0]:,} rows | {combined_df.shape[1]} columns")

    return combined_df

def main():
    df = load_multiple_csvs(CSV_DIR, limit=1)

    book = build_book(df)
    rating = build_rating(df, book)
    author, book_author = build_author_and_link(df, book)
    genre, book_genre = build_genre_and_link(df, book)

    print("\n✅ DataFrames criados:")
    print("book:", book.shape)
    print("rating:", rating.shape)
    print("author:", author.shape)
    print("genre:", genre.shape)
    print("book_author:", book_author.shape)
    print("book_genre:", book_genre.shape)

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    book.to_csv(OUT_DIR / "book.csv", index=False)
    rating.to_csv(OUT_DIR / "rating.csv", index=False)
    author.to_csv(OUT_DIR / "author.csv", index=False)
    genre.to_csv(OUT_DIR / "genre.csv", index=False)
    book_author.to_csv(OUT_DIR / "book_author.csv", index=False)
    book_genre.to_csv(OUT_DIR / "book_genre.csv", index=False)

    ddl = generate_postgres_ddl()
    (OUT_DIR / "schema.sql").write_text(ddl, encoding="utf-8")

    print("\n📦 CSVs e schema gerados com sucesso.")


if __name__ == "__main__":
    main()