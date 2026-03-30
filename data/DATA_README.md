# 📚 Data Pipeline Documentation

This document explains the full process of preparing, normalizing, and importing book data into the database.

---

## 🔹 Overview

The pipeline consists of three main stages:

1. **Compile raw CSV files into a single dataset**
2. **Normalize the compiled dataset into relational tables**
3. **Import the normalized data into PostgreSQL databases (microservices)**

---

## 📁 Folder Structure

```
data/
├── dataset/
│   ├── csvs/                 # Raw CSV files (one per genre)
│   ├── compile_all_books.py  # Script to merge all CSVs
│   └── compiled_books.csv    # Output of compilation
│
├── data_clean/
│   ├── normalize/            # Normalization scripts
│   ├── normalized_out/       # Output CSVs (tables)
│   └── db/                   # Import Database scripts
```

---

## ⚙️ Step 1 — Compile Raw CSV Files

Navigate to the dataset folder:

```bash
cd data/dataset
```

Run the compilation script:

```bash
python compile_all_books.py --limit X
```

- `X` = maximum number of CSV files to process
- If omitted, all files are processed

### ✅ Output

```
compiled_books.csv
```

This file contains all merged data from the raw dataset.

---

## 🔄 Step 2 — Normalize Data

Navigate to the normalization folder:

```bash
cd data/data_clean/normalize
```

Run the normalization script:

```bash
./rebuild_from_compiled.sh
```

### What happens here:

- Reads `compiled_books.csv`
- Cleans and structures the data
- Splits it into multiple CSV files (one per table)

### ✅ Output (in `normalized_out/`)

- `book.csv`
- `author.csv`
- `book_author.csv`
- `genre.csv`
- `book_genre.csv`
- `rating.csv`

---

## 🗄️ Step 3 — Import into Databases

Navigate to the import folder:

```bash
cd data/data_clean/db
```

Each service has its own script:

- `import_book.sh`
- `import_author.sh`
- `import_genre.sh`
- `import_rating.sh`

---

## 🚀 Import Command

```bash
./import_<SERVICE>.sh [-D] DB_USER DB_PASSWORD DB_PORT DB_NAME [CHUNK_SIZE]
```

### Parameters

- `SERVICE` → book | author | genre | rating
- `DB_USER` → database username
- `DB_PASSWORD` → database password
- `DB_PORT` → database port (e.g. 5432)
- `DB_NAME` → database name
- `CHUNK_SIZE` → optional (default: 10000)

---

## ⚠️ Optional Flag

### `-D` → Drop (truncate) before import

- Clears existing data before inserting new data

---

## 📌 Examples

### Import ratings (safe, no deletion)

```bash
./import_rating.sh bookcloud secret123 5432 bookcloud_db 10000
```

### Import ratings (clean before import)

```bash
./import_rating.sh -D bookcloud secret123 5432 bookcloud_db 10000
```

---

## ✅ Recommended Full Workflow

```bash
# 1. Compile data
cd data/dataset
python compile_all_books.py

# 2. Normalize
cd ../data_clean/normalize
./rebuild_from_compiled.sh

# 3. Import (example order)
cd ../db

./import_book.sh -D user pass 5432 db 10000
./import_author.sh -D user pass 5432 db 10000
./import_genre.sh -D user pass 5432 db 10000
./import_rating.sh -D user pass 5432 db 10000
```
