# BookCloud


## Setup Pyhton data_clean
First ypu  must have a folder called csvs with all genre csv files

1. Create .venv

    python3 -m venv venv

2. Activate .venv

    source venv/bin/activate

3. Save installed packages to requirements.txt

    pip freeze > requirements.txt

4. Install packages from requirements.txt

    pip install -r requirements.txt

5. Close .venv
deactivate




MAX_CHUNKS_PER_FILE=1 CHUNKSIZE=10000 python index.py


testes keycloak + kong:

```shell
echo "--- 1. No token → 401 ---"
http --headers GET $BASE/api/books

echo "--- 2. readonly GET /books → 200 ---"
http --headers GET $BASE/api/books "Authorization:Bearer $TOKEN_READONLY"

echo "--- 3. readonly POST /book → 403 ---"
http --headers POST $BASE/api/book \
  "Authorization:Bearer $TOKEN_READONLY" \
  isbn:=9999 name="Test Book" pub_year:=2024

echo "--- 4. readonly DELETE → 403 ---"
http --headers DELETE $BASE/api/book/9999 \
  "Authorization:Bearer $TOKEN_READONLY"

echo "--- 5. user POST /book → 200 ---"
http --headers POST $BASE/api/book \
  "Authorization:Bearer $TOKEN_USER" \
  isbn:=9999 name="Test Book" url="http://example.com" pub_year:=2024

echo "--- 6. user DELETE → 403 ---"
http --headers DELETE $BASE/api/book/9999 \
  "Authorization:Bearer $TOKEN_USER"

echo "--- 7. admin DELETE → 200 ---"
http --headers DELETE $BASE/api/book/9999 \
  "Authorization:Bearer $TOKEN_ADMIN"

echo "--- 8. all roles → analytics 200 ---"
http --headers GET $BASE/api/author-analytics/rank \
  "Authorization:Bearer $TOKEN_READONLY"

http --headers GET $BASE/api/author-analytics/rank \
  "Authorization:Bearer $TOKEN_USER"

http --headers GET $BASE/api/author-analytics/rank \
  "Authorization:Bearer $TOKEN_ADMIN"
```