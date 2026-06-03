# BookCloud API Gateway — Authentication & Routing

## Overview

BookCloud is a microservices-based book management application deployed on Kubernetes (minikube). This document covers the authentication architecture, role-based access control, routing implementation, and operational procedures for the API Gateway.

---

## Architecture

```
Client → Kong (API Gateway, port 9000)
           │
           ├── JWT validation (Keycloak RS256 public key)
           ├── Rate limiting (100 req/min, 1000 req/hour)
           ├── CORS
           │
           └── api-gateway (actix-web, port 8080)
                    │
                    ├── auth middleware (require_any_role)
                    ├── role checks in handlers (user/admin)
                    │
                    ├── book-catalog      (gRPC, Python)
                    ├── author-catalog    (gRPC, Rust)
                    ├── rating-catalog    (gRPC, Rust)
                    ├── book-search       (gRPC, Rust)
                    ├── book-recommendation (gRPC, Rust)
                    ├── compare-service   (gRPC, Python)
                    ├── genre-analysis    (gRPC, Python)
                    └── author-analytics  (gRPC, Rust)
```

---

## Stack

| Component         | Technology                        |
| ----------------- | --------------------------------- |
| API Gateway       | Rust (actix-web 4)                |
| Auth Gateway      | Kong 3.7 (declarative mode)       |
| Identity Provider | Keycloak (OIDC / JWT RS256)       |
| Microservices     | Rust (tonic gRPC) + Python (grpc) |
| Orchestration     | Kubernetes (minikube)             |
| Monitoring        | Prometheus + Grafana              |

---

## Authentication Flow

1. Client requests a JWT token from Keycloak
2. Client sends request to Kong with `Authorization: Bearer <token>`
3. Kong validates the JWT signature using Keycloak's RS256 public key
4. Kong proxies the request to the `api-gateway` service
5. `api-gateway` middleware (`require_any_role`) re-validates the token and extracts roles from `realm_access.roles`
6. Handlers with write/delete operations perform additional role checks

```
Keycloak Token Claims (realm_access.roles):
  readonlyuser → ["readonly"]
  testuser     → ["user"]
  adminuser    → ["admin"]
```

---

## Role-Based Access Control

| Endpoint                      | readonly | user  | admin |
| ----------------------------- | -------- | ----- | ----- |
| `GET /api/books`              | ✅        | ✅     | ✅     |
| `GET /api/book/{isbn}`        | ✅        | ✅     | ✅     |
| `POST /api/book`              | ❌ 403    | ✅     | ✅     |
| `PUT /api/book`               | ❌ 403    | ✅     | ✅     |
| `DELETE /api/book/{isbn}`     | ❌ 403    | ❌ 403 | ✅     |
| `GET /api/authors`            | ✅        | ✅     | ✅     |
| `GET /api/author/{id}`        | ✅        | ✅     | ✅     |
| `DELETE /api/author/{id}`     | ❌ 403    | ❌ 403 | ✅     |
| `GET /api/ratings`            | ✅        | ✅     | ✅     |
| `POST /api/rating`            | ❌ 403    | ✅     | ✅     |
| `GET /api/author-analytics/*` | ✅        | ✅     | ✅     |
| `GET /api/health`             | 🔓 public | 🔓     | 🔓     |

---

## Routing Architecture (actix-web)

### Starting the Cluster

```bash
./k8s/startup_service.sh
```

The script automatically:
1. Starts minikube with the `bookcloud` profile
2. Points the Docker CLI to minikube's internal daemon
3. Builds all service images in parallel inside minikube
4. Applies all Kubernetes manifests via `kubectl apply -k`
5. Bootstraps Keycloak — creates users and assigns roles
6. Fetches Keycloak's RS256 public key from JWKS and patches the Kong ConfigMap
7. Restarts Kong with the live public key

### Preparing Test Tokens

```bash
source ./k8s/test_setup.sh
```

This exports:
- `$BASE` — Kong URL (http://localhost:9000)
- `$TOKEN_READONLY` — JWT for `readonlyuser`
- `$TOKEN_USER` — JWT for `testuser`
- `$TOKEN_ADMIN` — JWT for `adminuser`

---

## API Usage Examples

### Get all books
```bash
http GET $BASE/api/books "Authorization:Bearer $TOKEN_READONLY"
```

### Create a book (user or admin only)
```bash
http POST $BASE/api/book \
  "Authorization:Bearer $TOKEN_USER" \
  isbn="9780000000001" \
  name="The Rust Programming Language" \
  url="https://doc.rust-lang.org/book/" \
  pub_year:=2019
```

### Delete a book (admin only)
```bash
http DELETE $BASE/api/book/9780000000001 \
  "Authorization:Bearer $TOKEN_ADMIN"
```

### Author analytics (all roles)
```bash
http GET $BASE/api/author-analytics/rank \
  "Authorization:Bearer $TOKEN_READONLY"
```

---

## Kubernetes Deployment Notes

### Image Pull Policy

Set `imagePullPolicy: IfNotPresent` in `k8s/api-gateway/deployment.yaml` when using minikube. `Always` causes Kubernetes to attempt an external registry pull, ignoring locally built images.

### Rebuilding the api-gateway

```bash
# Point Docker CLI to minikube's daemon
eval $(minikube -p bookcloud docker-env)

# Build (uses Cargo cache from previous builds)
docker build -t bookcloud/api-gateway:latest src/api-gateway

# Roll out
kubectl rollout restart deployment/api-gateway -n bookcloud
kubectl rollout status deployment/api-gateway -n bookcloud
```

### Bypassing Kong for debugging

```bash
POD=$(kubectl get pod -n bookcloud -l app=api-gateway -o jsonpath='{.items[0].metadata.name}')
kubectl port-forward pod/$POD 8082:8080 -n bookcloud &

# Test directly against actix-web
curl -s -w "\nHTTP: %{http_code}\n" \
  -H "Authorization: Bearer $TOKEN_USER" \
  http://localhost:8082/api/books
```

---

## Kong Configuration

Kong runs in **declarative (DB-less) mode** via a ConfigMap. The JWT plugin is applied to the `protected-routes` route (path `/`). The Keycloak RS256 public key is injected at cluster startup by `startup_service.sh` — the ConfigMap source file contains placeholders (`KONG_JWT_ISSUER_PLACEHOLDER`, `KONG_JWT_PUBLIC_KEY_PLACEHOLDER`) that are replaced in-memory at deploy time.

Active plugins:
- `jwt` — validates RS256 tokens against Keycloak's public key
- `rate-limiting` — 100 req/min, 1000 req/hour
- `cors` — allows all origins, GET/POST/PUT/DELETE/OPTIONS
- `request-size-limiting` — max 10MB payload
- `response-transformer` — adds `X-Powered-By: BookCloud` header

---

## Troubleshooting

| Symptom                                       | Cause                                                      | Fix                                                                                                   |
| --------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `404` on all routes via Kong                  | Kong port-forward died                                     | `source ./k8s/test_setup.sh`                                                                          |
| `404` directly on pod with valid token        | actix `web::resource` outside `web::scope` with middleware | Ensure all resources are registered inside the scope that wraps the middleware                        |
| `400` instead of `403` for readonly POST      | `web::Json<T>` deserializes before handler runs            | Use `web::Bytes` + manual deserialization after role check                                            |
| Pod running old binary after rebuild          | Docker BuildKit cache in WSL2 / `imagePullPolicy: Always`  | `docker build --no-cache` or `--build-arg CACHEBUST=$(date +%s)`; set `imagePullPolicy: IfNotPresent` |
| WSL crashes during `docker build`             | Rust compilation OOM                                       | Set `memory=4GB` in `.wslconfig`; use `cargo build --release` natively then `docker build`            |
| `minikube start` fails: over-alloc memory     | Existing profile has hardcoded memory                      | `minikube -p bookcloud delete` then restart with `BOOKCLOUD_MINIKUBE_MEMORY=4096`                     |
| `401 Missing or invalid Authorization header` | Token expired (5 min TTL)                                  | `source ./k8s/test_setup.sh` to refresh tokens                                                        |
