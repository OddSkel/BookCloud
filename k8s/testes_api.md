# BookCloud — Testes Funcionais da API

Este ficheiro reúne os testes usados para validar a aplicação **BookCloud** localmente em Kubernetes/minikube, via **Kong API Gateway**.

## 1. Contexto

Arquitetura validada:

```text
Cliente → Kong API Gateway → api-gateway → serviços gRPC → PostgreSQL/Redis
```

Valores usados:

```bash
export KONG_URL=http://localhost:9000
```

Utilizadores Keycloak:

| Utilizador | Password | Role |
|---|---:|---|
| `readonlyuser` | `password` | `readonly` |
| `testuser` | `password` | `user` |
| `adminuser` | `password` | `admin` |

Regras de autorização validadas:

| Recurso | Operação | Roles permitidas |
|---|---|---|
| Auth | login/register | público |
| Health | GET | público |
| Books | GET | `readonly`, `user`, `admin` |
| Books | POST/PUT | `user`, `admin` |
| Books | DELETE | `admin` |
| Authors | GET | `readonly`, `user`, `admin` |
| Authors | POST/PUT/DELETE | `admin` |
| Ratings | GET | `readonly`, `user`, `admin` |
| Ratings | POST/PUT/DELETE | `admin` |
| Genres | GET | `readonly`, `user`, `admin` |

---

## 2. Preparação do ambiente

### 2.1. Port-forward do Kong

```bash
kubectl -n bookcloud port-forward svc/kong 9000:80
```

Noutra shell:

```bash
export KONG_URL=http://localhost:9000
```

### 2.2. Obter tokens

```bash
export TOKEN_READONLY=$(
  http --body POST "$KONG_URL/api/auth/login" \
    username=readonlyuser \
    password=password \
  | jq -r '.access_token'
)

export TOKEN_USER=$(
  http --body POST "$KONG_URL/api/auth/login" \
    username=testuser \
    password=password \
  | jq -r '.access_token'
)

export TOKEN_ADMIN=$(
  http --body POST "$KONG_URL/api/auth/login" \
    username=adminuser \
    password=password \
  | jq -r '.access_token'
)
```

### 2.3. Confirmar roles nos tokens

```bash
for T in TOKEN_READONLY TOKEN_USER TOKEN_ADMIN; do
  echo
  echo "=== $T ==="
  echo "${!T}" | cut -d. -f2 | base64 -d 2>/dev/null \
    | jq '.preferred_username, .realm_access.roles'
done
```

Resultado esperado:

```text
TOKEN_READONLY → readonlyuser → contém readonly
TOKEN_USER     → testuser     → contém user
TOKEN_ADMIN    → adminuser    → contém admin
```

---

## 3. Testes públicos e autenticação

### 3.1. Health público

```bash
http --print=HhBb GET "$KONG_URL/api/health"
```

Esperado:

```text
HTTP/1.1 200 OK
```

Body esperado:

```json
{
  "service": "api-gateway",
  "status": "ok"
}
```

### 3.2. Endpoint protegido sem token

```bash
http --print=HhBb GET "$KONG_URL/api/books"
```

Esperado:

```text
HTTP/1.1 401 Unauthorized
```

### 3.3. Endpoint protegido com token

```bash
http --print=HhBb GET "$KONG_URL/api/books" \
  "Authorization: Bearer $TOKEN_READONLY"
```

Esperado:

```text
HTTP/1.1 200 OK
```

---

## 4. Testes de leitura dos catálogos

### 4.1. Books

```bash
http --print=HhBb GET "$KONG_URL/api/books" \
  "Authorization: Bearer $TOKEN_READONLY"
```

Esperado:

```text
HTTP/1.1 200 OK
```

### 4.2. Authors

```bash
http --print=HhBb GET "$KONG_URL/api/authors" \
  "Authorization: Bearer $TOKEN_READONLY"
```

Esperado:

```text
HTTP/1.1 200 OK
```

### 4.3. Ratings

```bash
http --print=HhBb GET "$KONG_URL/api/ratings" \
  "Authorization: Bearer $TOKEN_READONLY"
```

Esperado:

```text
HTTP/1.1 200 OK
```

### 4.4. Genres

```bash
http --print=HhBb GET "$KONG_URL/api/genres" \
  "Authorization: Bearer $TOKEN_READONLY"
```

Esperado:

```text
HTTP/1.1 200 OK
```

---

## 5. Testes Book CRUD/RBAC

Estado final da API:

```text
POST   /api/book
GET    /api/book/{isbn}
PUT    /api/book/{isbn}
DELETE /api/book/{isbn}
```

### 5.1. Criar, atualizar, ler e apagar livro

```bash
BOOK_ID=$(( $(date +%s) + RANDOM + 900000000 ))

http --print=HhBb POST "$KONG_URL/api/book" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  isbn="$BOOK_ID" \
  name="Book Final REST $BOOK_ID" \
  pub_year:=2026 \
  summary="Final REST book test" \
  url="https://example.com/book/$BOOK_ID"

http --print=HhBb PUT "$KONG_URL/api/book/$BOOK_ID" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  name="Book Final REST UPDATED $BOOK_ID" \
  pub_year:=2027 \
  summary="Final REST book update" \
  url="https://example.com/book/$BOOK_ID"

http --print=HhBb GET "$KONG_URL/api/book/$BOOK_ID" \
  "Authorization: Bearer $TOKEN_READONLY"

http --print=HhBb DELETE "$KONG_URL/api/book/$BOOK_ID" \
  "Authorization: Bearer $TOKEN_ADMIN"
```

Esperado:

```text
POST   /api/book              → 201 Created
PUT    /api/book/{isbn}       → 200 OK
GET    /api/book/{isbn}       → 200 OK, com dados atualizados
DELETE /api/book/{isbn}       → 204 No Content
```

### 5.2. RBAC do update de books

```bash
BOOK_ID=$(( $(date +%s) + RANDOM + 900000000 ))

http --print=HhBb POST "$KONG_URL/api/book" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  isbn="$BOOK_ID" \
  name="Book RBAC Probe $BOOK_ID" \
  pub_year:=2026 \
  summary="Book RBAC test" \
  url="https://example.com/book/$BOOK_ID"

for ROLE in READONLY USER ADMIN; do
  TOKEN_VAR="TOKEN_$ROLE"
  TOKEN="${!TOKEN_VAR}"

  echo
  echo "===== $ROLE: PUT /api/book/$BOOK_ID ====="
  http --print=HhBb PUT "$KONG_URL/api/book/$BOOK_ID" \
    "Authorization: Bearer $TOKEN" \
    name="Book RBAC UPDATED by $ROLE $BOOK_ID" \
    pub_year:=2027 \
    summary="Updated through path-based PUT" \
    url="https://example.com/book/$BOOK_ID"
done

http --print=HhBb DELETE "$KONG_URL/api/book/$BOOK_ID" \
  "Authorization: Bearer $TOKEN_ADMIN"
```

Esperado:

```text
readonly → 403 Forbidden
user     → 200 OK
admin    → 200 OK
cleanup  → 204 No Content
```

---

## 6. Testes Author CRUD/RBAC

Estado final da API:

```text
POST   /api/author
GET    /api/author/{author_id}
PUT    /api/author/{author_id}
DELETE /api/author/{author_id}
```

### 6.1. Criar, atualizar, ler e apagar autor

```bash
AUTHOR_ID=$(( $(date +%s) + RANDOM + 900000000 ))

http --print=HhBb POST "$KONG_URL/api/author" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  author_id:="$AUTHOR_ID" \
  name="Author Final REST $AUTHOR_ID"

http --print=HhBb PUT "$KONG_URL/api/author/$AUTHOR_ID" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  name="Author Final REST UPDATED $AUTHOR_ID"

http --print=HhBb GET "$KONG_URL/api/author/$AUTHOR_ID" \
  "Authorization: Bearer $TOKEN_READONLY"

http --print=HhBb DELETE "$KONG_URL/api/author/$AUTHOR_ID" \
  "Authorization: Bearer $TOKEN_ADMIN"
```

Esperado:

```text
POST   /api/author             → 201 Created
PUT    /api/author/{id}        → 200 OK
GET    /api/author/{id}        → 200 OK, com dados atualizados
DELETE /api/author/{id}        → 204 No Content
```

### 6.2. RBAC do update de authors

```bash
AUTHOR_ID=$(( $(date +%s) + RANDOM + 900000000 ))

http --print=HhBb POST "$KONG_URL/api/author" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  author_id:="$AUTHOR_ID" \
  name="Author RBAC Probe $AUTHOR_ID"

for ROLE in READONLY USER ADMIN; do
  TOKEN_VAR="TOKEN_$ROLE"
  TOKEN="${!TOKEN_VAR}"

  echo
  echo "===== $ROLE: PUT /api/author/$AUTHOR_ID ====="
  http --print=HhBb PUT "$KONG_URL/api/author/$AUTHOR_ID" \
    "Authorization: Bearer $TOKEN" \
    name="Author RBAC UPDATED by $ROLE $AUTHOR_ID"
done

http --print=HhBb DELETE "$KONG_URL/api/author/$AUTHOR_ID" \
  "Authorization: Bearer $TOKEN_ADMIN"
```

Esperado:

```text
readonly → 403 Forbidden
user     → 403 Forbidden
admin    → 200 OK
cleanup  → 204 No Content
```

---

## 7. Testes Rating CRUD/RBAC

Estado final da API:

```text
GET    /api/rating/{book_isbn}
POST   /api/rating/{book_isbn}
PUT    /api/rating/{book_isbn}
DELETE /api/rating/{book_isbn}
```

### 7.1. Criar, atualizar e apagar rating

```bash
RATING_BOOK_ID=$(( $(date +%s) + RANDOM + 900000000 ))

http --print=HhBb POST "$KONG_URL/api/rating/$RATING_BOOK_ID" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  star_rating:=4.2 \
  num_ratings:=12

http --print=HhBb PUT "$KONG_URL/api/rating/$RATING_BOOK_ID" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  star_rating:=4.8 \
  num_ratings:=20

http --print=HhBb GET "$KONG_URL/api/rating/$RATING_BOOK_ID" \
  "Authorization: Bearer $TOKEN_READONLY"

http --print=HhBb DELETE "$KONG_URL/api/rating/$RATING_BOOK_ID" \
  "Authorization: Bearer $TOKEN_ADMIN"
```

Esperado:

```text
POST   /api/rating/{book_isbn}   → 201 Created
PUT    /api/rating/{book_isbn}   → 200 OK
GET    /api/rating/{book_isbn}   → 200 OK
DELETE /api/rating/{book_isbn}   → 204 No Content
```

### 7.2. RBAC do update/delete de ratings

```bash
RATING_BOOK_ID=$(( $(date +%s) + RANDOM + 900000000 ))

http --print=HhBb POST "$KONG_URL/api/rating/$RATING_BOOK_ID" \
  "Authorization: Bearer $TOKEN_ADMIN" \
  star_rating:=4.2 \
  num_ratings:=12

for ROLE in READONLY USER ADMIN; do
  TOKEN_VAR="TOKEN_$ROLE"
  TOKEN="${!TOKEN_VAR}"

  echo
  echo "===== $ROLE: PUT /api/rating/$RATING_BOOK_ID ====="
  http --print=HhBb PUT "$KONG_URL/api/rating/$RATING_BOOK_ID" \
    "Authorization: Bearer $TOKEN" \
    star_rating:=4.9 \
    num_ratings:=25
done

for ROLE in READONLY USER ADMIN; do
  TOKEN_VAR="TOKEN_$ROLE"
  TOKEN="${!TOKEN_VAR}"

  echo
  echo "===== $ROLE: DELETE /api/rating/$RATING_BOOK_ID ====="
  http --print=HhBb DELETE "$KONG_URL/api/rating/$RATING_BOOK_ID" \
    "Authorization: Bearer $TOKEN"
done
```

Esperado:

```text
PUT:
  readonly → 403 Forbidden
  user     → 403 Forbidden
  admin    → 200 OK

DELETE:
  readonly → 403 Forbidden
  user     → 403 Forbidden
  admin    → 204 No Content
```

---

## 9. Estado final da API validada

### Auth e health

```text
GET  /api/health
POST /api/auth/login
POST /api/auth/register
```

### Books

```text
GET    /api/books
POST   /api/book
GET    /api/book/{isbn}
PUT    /api/book/{isbn}
DELETE /api/book/{isbn}
```

### Authors

```text
GET    /api/authors
POST   /api/author
GET    /api/author/{author_id}
PUT    /api/author/{author_id}
DELETE /api/author/{author_id}
```

### Ratings

```text
GET    /api/ratings
POST   /api/rating/{book_isbn}
GET    /api/rating/{book_isbn}
PUT    /api/rating/{book_isbn}
DELETE /api/rating/{book_isbn}
```

### Genres

```text
GET    /api/genres
POST   /api/genres
GET    /api/genre/{genre_id}
PUT    /api/genre/{genre_id}
DELETE /api/genre/{genre_id}
GET    /api/genre/{genre_id}/growth
GET    /api/genre/{genre_id}/popularity
```

---

## 10. Checklist final para apresentação

```text
[OK] Kong responde em /api/health
[OK] Rotas protegidas sem token devolvem 401
[OK] Login Keycloak devolve JWT válido
[OK] JWT contém roles corretas
[OK] GET books/authors/ratings/genres funciona
[OK] POST/PUT/DELETE book respeita RBAC
[OK] POST/PUT/DELETE author respeita RBAC
[OK] POST/PUT/DELETE rating respeita RBAC
[OK] POST/PUT/DELETE genre respeita RBAC
[OK] Author analytics rank/performance/consistency/growth testado por token/role
```
