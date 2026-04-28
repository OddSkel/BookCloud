#!/bin/bash
set -e

echo "=== BookCloud Database Setup ==="
echo ""

echo "[1/4] Populating rating-db..."
./import_rating.sh -D user password 5434 db 10000
echo "✔ rating-db done"

echo ""
echo "[2/4] Populating genre-db..."
./import_genre.sh -D user password 5433 db 10000
echo "✔ genre-db done"

echo ""
echo "[3/4] Populating book-db..."
./import_book.sh -D user password 5432 db 10000
echo "✔ book-db done"

echo ""
echo "[4/4] Populating author-db..."
./import_author.sh -D user password 5431 db 10000
echo "✔ author-db done"

echo ""
echo "=== Verification ==="
docker exec book-db   psql -U user -d db -c "SELECT COUNT(*) FROM book;"
docker exec author-db psql -U user -d db -c "SELECT COUNT(*) FROM author;"
docker exec rating-db psql -U user -d db -c "SELECT COUNT(*) FROM rating;"
docker exec genre-db  psql -U user -d db -c "SELECT COUNT(*) FROM genre;"

echo ""
echo "=== Setup complete! ==="
