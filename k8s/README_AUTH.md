# BookCloud API Gateway — Authentication & Routing

## Overview

BookCloud is a microservices-based book management application deployed on Kubernetes using a local `minikube` profile. This document covers the authentication architecture, Keycloak bootstrap process, user registration flow, role-based access control, Kong routing, and operational procedures for the API Gateway.

The current authentication stack uses:

- **Kong** as the external API Gateway on `http://localhost:9000`
- **Keycloak** as the identity provider on `http://localhost:8000` during bootstrap/debug
- **api-gateway** as the internal Rust/Actix service
- **JWT RS256** tokens issued by Keycloak and validated by Kong
- **Realm roles** in the Keycloak realm `bookcloud`

---

## Architecture

```text
Client → Kong (API Gateway, port-forward localhost:9000 → svc/kong:80)
           │
           ├── Public routes
           │     ├── POST /api/auth/login
           │     ├── POST /api/auth/register
           │     └── GET  /api/health
           │
           ├── Protected routes
           │     └── JWT validation with Keycloak RS256 public key
           │
           ├── Global plugins
           │     ├── Rate limiting: 100 req/min, 1000 req/hour
           │     ├── CORS
           │     ├── Request size limiting
           │     └── X-Powered-By response header
           │
           └── api-gateway (Rust / Actix-web, service port 80)
                    │
                    ├── auth middleware / role checks
                    ├── register handler using Keycloak Admin API
                    │
                    ├── book-catalog            (gRPC)
                    ├── author-catalog          (gRPC)
                    ├── rating-catalog          (gRPC)
                    ├── book-search             (gRPC)
                    ├── book-recommendation     (gRPC)
                    ├── compare-service         (gRPC)
                    ├── genre-analysis-service  (gRPC)
                    └── author-analytics-service(gRPC)
```

---

## Stack

| Component         | Technology                        |
| ----------------- | --------------------------------- |
| API service       | Rust / Actix-web                  |
| API Gateway       | Kong 3.7, DB-less/declarative     |
| Identity Provider | Keycloak, OIDC/JWT RS256          |
| Microservices     | Rust tonic gRPC + Python gRPC     |
| Orchestration     | Kubernetes / minikube             |
| Monitoring        | Prometheus + Grafana              |

---

## Runtime URLs

| Component | URL |
| --------- | --- |
| Kong | `http://localhost:9000` |
| Keycloak bootstrap/debug | `http://localhost:8000` |
| Keycloak internal service | `http://keycloak:80` |
| api-gateway internal service | `http://api-gateway:80` |

Required port-forwards during manual testing:

```bash
kubectl -n bookcloud port-forward svc/kong 9000:80
kubectl -n bookcloud port-forward svc/keycloak 8000:80
```

---

## Keycloak Realm and Client

| Item | Value |
| ---- | ----- |
| Realm | `bookcloud` |
| Client | `bookcloud-app` |
| Client secret | `bookcloud-app-secret` |
| Token endpoint | `/realms/bookcloud/protocol/openid-connect/token` |
| Admin users endpoint | `/admin/realms/bookcloud/users` |

The `bookcloud-app` client secret is intentionally fixed as:

```text
bookcloud-app-secret
```

The startup script must not call:

```text
POST /admin/realms/bookcloud/clients/{CLIENT_UUID}/client-secret
```

That endpoint rotates/generates a new secret and breaks both Kong login and the api-gateway registration flow.

---

## Realm Roles

BookCloud uses **realm roles** in the `bookcloud` realm:

| Role | Purpose |
| ---- | ------- |
| `readonly` | Read-only access |
| `user` | Standard user access |
| `admin` | Administrative access |

Roles appear in JWTs under:

```json
{
  "realm_access": {
    "roles": ["user"]
  }
}
```

The api-gateway reads `realm_access.roles` and authorizes handlers according to the endpoint rules.

---

## Bootstrap Users

The startup script creates three test users:

| Username | Password | Realm role |
| -------- | -------- | ---------- |
| `testuser` | `password` | `user` |
| `adminuser` | `password` | `admin` |
| `readonlyuser` | `password` | `readonly` |

These users are kept as fixtures for manual and automated RBAC testing. They are not required for normal application registration, but they are useful for quickly validating role behavior.

---

## Service Account Permissions

The `bookcloud-app` service account is used by the api-gateway to create users through the Keycloak Admin API.

Service account user:

```text
service-account-bookcloud-app
```

Required `realm-management` client roles:

| Role | Why it is needed |
| ---- | ---------------- |
| `manage-users` | Create users and assign role mappings |
| `view-users` | Read user details |
| `query-users` | Search users by username |
| `view-realm` | Read realm roles such as `/roles/user` |

Without `view-realm`, registration can create the user but fails when assigning the default `user` role:

```json
{
  "error": "user_created_but_role_lookup_failed"
}
```

---

## Authentication Flow

### Login

Client request:

```http
POST /api/auth/login
Content-Type: application/x-www-form-urlencoded

username=testuser&password=password
```

Kong handles `/api/auth/login` as a public route and forwards it to Keycloak:

```text
http://keycloak:80/realms/bookcloud/protocol/openid-connect/token
```

The Kong `request-transformer` injects:

```text
grant_type=password
client_id=bookcloud-app
client_secret=bookcloud-app-secret
```

Keycloak returns:

```json
{
  "access_token": "...",
  "refresh_token": "...",
  "token_type": "Bearer",
  "expires_in": 300
}
```

The returned access token has issuer:

```text
http://localhost:8000/realms/bookcloud
```

Kong validates protected-route JWTs by matching the token `iss` claim against the JWT credential key configured in `kong-config`.

---

## User Registration Flow

Client request:

```http
POST /api/auth/register
Content-Type: application/json

{
  "username": "newuser...",
  "password": "password",
  "email": "newuser@example.com"
}
```

The route `/api/auth/register` is public in Kong and is proxied to `api-gateway`.

The api-gateway then:

1. Requests an admin/service-account token from Keycloak using `client_credentials`:

   ```text
   POST /realms/bookcloud/protocol/openid-connect/token
   client_id=bookcloud-app
   client_secret=bookcloud-app-secret
   grant_type=client_credentials
   ```

2. Converts the public register payload into a Keycloak `UserRepresentation`.

   Input received by the api-gateway:

   ```json
   {
     "username": "newuser...",
     "password": "password",
     "email": "newuser@example.com"
   }
   ```

   Payload sent to Keycloak:

   ```json
   {
     "username": "newuser...",
     "email": "newuser@example.com",
     "enabled": true,
     "credentials": [
       {
         "type": "password",
         "value": "password",
         "temporary": false
       }
     ]
   }
   ```

   The field `password` must not be sent at the top level to Keycloak. It must be sent inside `credentials`.

3. Creates the user:

   ```text
   POST /admin/realms/bookcloud/users
   ```

4. Looks up the created user:

   ```text
   GET /admin/realms/bookcloud/users?username=<username>
   ```

5. Looks up the default realm role:

   ```text
   GET /admin/realms/bookcloud/roles/user
   ```

6. Assigns the default `user` role:

   ```text
   POST /admin/realms/bookcloud/users/{USER_ID}/role-mappings/realm
   ```

7. Returns:

   ```json
   {
     "message": "User created successfully",
     "role": "user"
   }
   ```

A newly registered user can then login immediately and receives a JWT containing the `user` role.

---

## Role-Based Access Control

| Endpoint                      | Public | readonly | user | admin |
| ----------------------------- | ------ | -------- | ---- | ----- |
| `POST /api/auth/login`        | ✅     | ✅       | ✅   | ✅    |
| `POST /api/auth/register`     | ✅     | ✅       | ✅   | ✅    |
| `GET /api/health`             | ✅     | ✅       | ✅   | ✅    |
| `GET /api/books`              | ❌     | ✅       | ✅   | ✅    |
| `GET /api/book/{isbn}`        | ❌     | ✅       | ✅   | ✅    |
| `POST /api/book`              | ❌     | ❌ 403   | ✅   | ✅    |
| `PUT /api/book`               | ❌     | ❌ 403   | ✅   | ✅    |
| `DELETE /api/book/{isbn}`     | ❌     | ❌ 403   | ❌ 403 | ✅  |
| `GET /api/authors`            | ❌     | ✅       | ✅   | ✅    |
| `GET /api/author/{id}`        | ❌     | ✅       | ✅   | ✅    |
| `DELETE /api/author/{id}`     | ❌     | ❌ 403   | ❌ 403 | ✅  |
| `GET /api/ratings`            | ❌     | ✅       | ✅   | ✅    |
| `POST /api/rating`            | ❌     | ❌ 403   | ✅   | ✅    |
| `GET /api/author-analytics/*` | ❌     | ✅       | ✅   | ✅    |

---

## Starting the Cluster

```bash
./k8s/startup_service.sh
```

The script:

1. Starts/reuses the `bookcloud` minikube profile.
2. Points Docker CLI to minikube's internal daemon.
3. Builds all service images in parallel inside minikube.
4. Installs/updates the local monitoring stack.
5. Applies Kubernetes manifests using `kubectl apply -k`.
6. Starts a temporary Keycloak port-forward on `localhost:8000`.
7. Waits for non-Kong deployments to roll out.
8. Fixes `bookcloud-app` client secret to `bookcloud-app-secret`.
9. Ensures service-account roles:
   - `manage-users`
   - `view-users`
   - `query-users`
   - `view-realm`
10. Creates bootstrap users:
    - `testuser`
    - `adminuser`
    - `readonlyuser`
11. Fetches Keycloak's RS256 public key from JWKS.
12. Replaces `KONG_JWT_ISSUER_PLACEHOLDER` and `KONG_JWT_PUBLIC_KEY_PLACEHOLDER` in Kong config.
13. Restarts Kong with the live issuer/public key.

---

## Preparing Test Tokens

If `k8s/test_setup.sh` is available:

```bash
source ./k8s/test_setup.sh
```

This typically exports:

- `$BASE` or `$KONG_URL` — Kong URL, usually `http://localhost:9000`
- `$TOKEN_READONLY` — JWT for `readonlyuser`
- `$TOKEN_USER` — JWT for `testuser`
- `$TOKEN_ADMIN` — JWT for `adminuser`

Manual token setup:

```bash
export KONG_URL=http://localhost:9000

TOKEN_USER=$(http -f POST "$KONG_URL/api/auth/login" \
  username=testuser \
  password=password | jq -r '.access_token')

TOKEN_ADMIN=$(http -f POST "$KONG_URL/api/auth/login" \
  username=adminuser \
  password=password | jq -r '.access_token')

TOKEN_READONLY=$(http -f POST "$KONG_URL/api/auth/login" \
  username=readonlyuser \
  password=password | jq -r '.access_token')
```

---

## API Usage Examples

### Register a new user

```bash
NEW_USER="newuser$(date +%s)"
NEW_EMAIL="$NEW_USER@example.com"

http POST "$KONG_URL/api/auth/register" \
  username="$NEW_USER" \
  password="password" \
  email="$NEW_EMAIL"
```

Expected response:

```json
{
  "message": "User created successfully",
  "role": "user"
}
```

### Login with a newly registered user

```bash
TOKEN_NEW=$(http -f POST "$KONG_URL/api/auth/login" \
  username="$NEW_USER" \
  password=password | jq -r '.access_token')

echo "$TOKEN_NEW" | cut -c1-80
```

### Inspect token roles

```bash
echo "$TOKEN_NEW" | cut -d. -f2 | base64 -d 2>/dev/null \
  | jq '.preferred_username, .realm_access.roles'
```

Expected role list includes:

```json
"user"
```

### Get all books

```bash
http GET "$KONG_URL/api/books" \
  "Authorization: Bearer $TOKEN_NEW"
```

If the book database is empty, the expected response is:

```json
[]
```

### Create a book, user or admin only

```bash
http POST "$KONG_URL/api/book" \
  "Authorization: Bearer $TOKEN_USER" \
  isbn="9780000000001" \
  name="The Rust Programming Language" \
  url="https://doc.rust-lang.org/book/" \
  summary_clean="Rust book" \
  pub_year:=2019
```

### Delete a book, admin only

```bash
http DELETE "$KONG_URL/api/book/9780000000001" \
  "Authorization: Bearer $TOKEN_ADMIN"
```

### Author analytics, all roles

```bash
http GET "$KONG_URL/api/author-analytics/rank" \
  "Authorization: Bearer $TOKEN_READONLY"
```

---

## Kong Configuration

Kong runs in **declarative DB-less mode** via the `kong-config` ConfigMap.

Important routes:

| Route | Path | Auth |
| ----- | ---- | ---- |
| `auth-login` | `POST /api/auth/login` | Public |
| `public-routes` | `/api/health`, `/api/auth/register` | Public |
| `protected-routes` | `/` | JWT required |

The `auth-login` route forwards to Keycloak's token endpoint and injects:

```text
grant_type=password
client_id=bookcloud-app
client_secret=bookcloud-app-secret
```

The JWT plugin is applied only to `protected-routes`.

The Kong source template must include:

```yaml
key: "KONG_JWT_ISSUER_PLACEHOLDER"
rsa_public_key: |
  KONG_JWT_PUBLIC_KEY_PLACEHOLDER
```

At startup, `startup_service.sh` replaces those placeholders in memory with:

```text
key: "http://localhost:8000/realms/bookcloud"
```

and Keycloak's live RS256 public key.

Do not leave:

```yaml
key: "null"
```

Otherwise protected requests fail with:

```json
{
  "message": "No credentials found for given 'iss'"
}
```

Active plugins:

- `jwt` — validates RS256 tokens against Keycloak's public key
- `rate-limiting` — 100 req/min, 1000 req/hour
- `cors` — allows all origins, GET/POST/PUT/DELETE/OPTIONS
- `request-size-limiting` — max 10MB payload
- `response-transformer` — adds `X-Powered-By: BookCloud`

---

## Kubernetes Deployment Notes

### Image Pull Policy

Set `imagePullPolicy: IfNotPresent` in local minikube deployments. `Always` can cause Kubernetes to attempt an external registry pull and ignore locally built images.

### Rebuilding the api-gateway

```bash
cd ~/mestradoFCUL/2ºsem/Comp_Cloud/BookCloud

eval "$(minikube -p bookcloud docker-env)"

TAG="api-gateway-$(date +%s)"
docker build -t "bookcloud/api-gateway:$TAG" ./src/api-gateway

kubectl -n bookcloud set image deployment/api-gateway \
  api-gateway="bookcloud/api-gateway:$TAG"

kubectl -n bookcloud rollout status deployment/api-gateway
```

### Bypassing Kong for debugging

```bash
POD=$(kubectl get pod -n bookcloud -l app=api-gateway -o jsonpath='{.items[0].metadata.name}')
kubectl port-forward pod/$POD 8082:8080 -n bookcloud &

curl -s -w "\nHTTP: %{http_code}\n" \
  -H "Authorization: Bearer $TOKEN_USER" \
  http://localhost:8082/api/books
```

---

## Verification Checklist

### Keycloak direct password login

```bash
curl -s -X POST "http://localhost:8000/realms/bookcloud/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=bookcloud-app" \
  -d "client_secret=bookcloud-app-secret" \
  -d "grant_type=password" \
  -d "username=testuser" \
  -d "password=password" | jq '.access_token != null'
```

Expected:

```text
true
```

### Keycloak service-account token

```bash
curl -s -X POST "http://localhost:8000/realms/bookcloud/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=bookcloud-app" \
  -d "client_secret=bookcloud-app-secret" \
  -d "grant_type=client_credentials" | jq '.access_token != null'
```

Expected:

```text
true
```

### Service account can read role `user`

```bash
ADMIN_TOKEN=$(curl -s -X POST "http://localhost:8000/realms/bookcloud/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=bookcloud-app" \
  -d "client_secret=bookcloud-app-secret" \
  -d "grant_type=client_credentials" | jq -r '.access_token')

curl -i "http://localhost:8000/admin/realms/bookcloud/roles/user" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

Expected:

```text
HTTP/1.1 200 OK
```

### Register, login, protected route

```bash
NEW_USER="newuser$(date +%s)"
NEW_EMAIL="$NEW_USER@example.com"

http POST "$KONG_URL/api/auth/register" \
  username="$NEW_USER" \
  password="password" \
  email="$NEW_EMAIL"

TOKEN_NEW=$(http -f POST "$KONG_URL/api/auth/login" \
  username="$NEW_USER" \
  password=password | jq -r '.access_token')

echo "$TOKEN_NEW" | cut -d. -f2 | base64 -d 2>/dev/null \
  | jq '.preferred_username, .realm_access.roles'

http GET "$KONG_URL/api/books" \
  "Authorization: Bearer $TOKEN_NEW"
```

Expected:

- register returns `201 Created`
- token includes `user`
- protected request returns `200 OK`

---

## Troubleshooting

| Symptom | Cause | Fix |
| ------- | ----- | --- |
| `POST /api/auth/login` returns `401 Unauthorized` | `/api/auth/login` is falling into Kong `protected-routes`, or Kong has wrong client secret | Ensure `auth-login` route exists and injects `client_secret=bookcloud-app-secret` |
| Login returns token, but protected route returns `No credentials found for given 'iss'` | Kong JWT credential key does not match token `iss`, often `key: "null"` | Use `KONG_JWT_ISSUER_PLACEHOLDER` in template and let startup script replace it |
| Protected route returns `Bad token; invalid JSON` | Token variable is empty, `null`, or malformed | Re-run login and confirm `echo "$TOKEN" | jq -R 'split(".") | length'` returns `3` |
| `POST /api/auth/register` returns `admin_token_failed` | api-gateway cannot obtain service-account token; wrong realm or wrong secret | Use `/realms/bookcloud/...`, set `KEYCLOAK_ADMIN_SECRET=bookcloud-app-secret`, and keep Keycloak client secret fixed |
| `POST /api/auth/register` returns `Unrecognized field "password"` | api-gateway sent `password` at top level to Keycloak | Send password inside `credentials[]` |
| `POST /api/auth/register` returns `user_created_but_role_lookup_failed` | service account lacks `view-realm` | Assign `view-realm` from `realm-management` to `service-account-bookcloud-app` |
| New user logs in but `GET /api/books` returns `403 Insufficient role` | user was created without realm role `user` | Assign `user` role during registration |
| `GET /api/books` returns `200 []` | Authentication is working; database may be empty | Seed/populate the book database |
| `404` on all routes via Kong | Kong port-forward died | Restart `kubectl -n bookcloud port-forward svc/kong 9000:80` |
| Pod running old binary after rebuild | Docker BuildKit cache or image tag not changed | Build with a unique tag and update deployment image |
| WSL crashes during `docker build` | Rust compilation OOM | Reduce minikube resources or increase WSL memory |
| `minikube start` fails due to profile/resource mismatch | Existing profile has old configuration | `minikube -p bookcloud delete` and rerun startup |
| Helm/Docker tries IPv6 and fails | Local network has broken IPv6 path | Disable IPv6 temporarily or force/pull dependencies before startup |

---

## Final Known-Good Auth State

A working deployment should satisfy all of these:

```text
POST /api/auth/login with testuser/password      → 200 + access_token
GET /api/books without token                     → 401 from Kong
GET /api/books with testuser token               → 200, possibly []
POST /api/auth/register with new username        → 201 + role user
POST /api/auth/login with newly registered user  → 200 + access_token
JWT of newly registered user contains            → realm_access.roles includes "user"
GET /api/books with newly registered user token  → 200, possibly []
```
