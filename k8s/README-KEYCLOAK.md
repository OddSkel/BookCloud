# Keycloak — Authentication & Authorization

Keycloak is deployed as the Identity Provider (IdP) for BookCloud, handling all authentication and token issuance for the cluster. It exposes an OAuth2/OpenID Connect interface and issues RS256-signed JWTs that Kong validates at the API Gateway before forwarding requests to any microservice.

---

## Architecture

```
Client
  │
  ├─> Keycloak (port-forward 8080) ──> issues JWT token
  │
  └─> Kong (port-forward 9000)
        │  validates JWT signature using Keycloak's public key
        └─> api-gateway ──> microservices
```

---

## Kubernetes Deployment

Keycloak runs in the `bookcloud` namespace alongside the other services.

| Resource | Name | Description |
|---|---|---|
| Deployment | `keycloak` | Single replica, `quay.io/keycloak/keycloak:24` |
| Service | `keycloak` | ClusterIP on port 80 |
| ConfigMap | `keycloak-config` | Environment variables |
| Secret | `keycloak-credentials` | Admin username and password |
| PVC | `keycloak-pvc` | Persistent storage for the Keycloak database |

### Apply

```bash
kubectl apply -f k8s/keycloak/
```

### Port-forward for local access

```bash
kubectl -n bookcloud port-forward service/keycloak 8080:80
```

The admin console is then available at: `http://localhost:8080`

---

## Realm Configuration

BookCloud uses a dedicated realm called `bookcloud`. All clients, users, and roles are scoped to this realm.

### Realm: `bookcloud`

| Setting | Value |
|---|---|
| Realm name | `bookcloud` |
| Token signing algorithm | RS256 |
| Access token TTL | 300 seconds (5 minutes) |
| Refresh token TTL | 1800 seconds (30 minutes) |

### Client: `bookcloud-app`

| Setting | Value |
|---|---|
| Client ID | `bookcloud-app` |
| Access type | Public |
| Direct Access Grants | Enabled (allows `grant_type=password`) |

---

## Obtaining a Token

Use the Resource Owner Password Credentials grant to obtain a JWT for testing:

```bash
curl -s -X POST http://localhost:8080/realms/bookcloud/protocol/openid-connect/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password" \
  -d "client_id=bookcloud-app" \
  -d "username=testuser" \
  -d "password=testpass" | python3 -m json.tool
```

The response contains:

| Field | Description |
|---|---|
| `access_token` | RS256-signed JWT, valid for 300 seconds |
| `refresh_token` | Used to obtain a new access token without re-authenticating |
| `expires_in` | Access token TTL in seconds |
| `token_type` | Always `Bearer` |

### Store the token in a variable

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/realms/bookcloud/protocol/openid-connect/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password" \
  -d "client_id=bookcloud-app" \
  -d "username=testuser" \
  -d "password=testpass" | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")
```

---

## JWT Token Structure

The access token is a standard JWT with three Base64-encoded sections: header, payload, and signature.

### Header

```json
{
  "alg": "RS256",
  "typ": "JWT",
  "kid": "9qaXUsTYo8KJWc1PtPYw-qvM27bwC9qyVGj075mRPo4"
}
```

The `kid` (Key ID) identifies which public key from the JWKS endpoint should be used to verify the signature.

### Payload (relevant claims)

| Claim | Value | Description |
|---|---|---|
| `iss` | `http://localhost:8080/realms/bookcloud` | Token issuer — must match Kong's JWT consumer `key` |
| `sub` | UUID | Subject — unique user identifier |
| `exp` | Unix timestamp | Token expiry |
| `azp` | `bookcloud-app` | Authorized party (client that requested the token) |
| `realm_access.roles` | `["default-roles-bookcloud", ...]` | Realm-level roles assigned to the user |
| `preferred_username` | `testuser` | Human-readable username |

---

## Public Key & JWKS

Keycloak exposes its public keys at the JWKS endpoint. Kong uses these to verify JWT signatures without contacting Keycloak on every request.

### JWKS endpoint

```bash
curl -s http://localhost:8080/realms/bookcloud/protocol/openid-connect/certs | python3 -m json.tool
```

The signing key has `"use": "sig"` and `"alg": "RS256"`. The encryption key has `"use": "enc"`.

### Get the public key in PEM format

```bash
curl -s http://localhost:8080/realms/bookcloud | python3 -c "
import sys, json, textwrap
data = json.load(sys.stdin)
key = data['public_key']
pem = '-----BEGIN PUBLIC KEY-----\n'
pem += textwrap.fill(key, 64)
pem += '\n-----END PUBLIC KEY-----'
print(pem)
"
```

---

## Kong Integration

Kong validates JWT tokens using the `jwt` plugin in DB-less mode. The consumer is configured with the Keycloak public key so Kong can verify signatures locally without calling Keycloak.

### Consumer configuration in `kong/configmap.yaml`

```yaml
consumers:
  - username: keycloak-users
    jwt_secrets:
      - key: "http://localhost:8080/realms/bookcloud"   # must match token `iss` claim
        algorithm: RS256
        rsa_public_key: |
          -----BEGIN PUBLIC KEY-----
          <Keycloak RS256 public key>
          -----END PUBLIC KEY-----
```

> **Important:** The `key` value must exactly match the `iss` claim in the token. When accessing Kong from inside the cluster, tokens will carry `iss: http://keycloak/realms/bookcloud` — update the consumer `key` accordingly and set `KC_HOSTNAME=keycloak` in the Keycloak ConfigMap.

### Testing Kong JWT validation

```bash
# Without token → 401 Unauthorized
http GET localhost:9000/api/books

# With valid token → proxied to api-gateway
http GET localhost:9000/api/books "Authorization: Bearer $TOKEN"
```

---

## User Management

Users are managed through the Keycloak Admin Console at `http://localhost:8080/admin`.

### Create a user via Admin Console

1. Navigate to **Realm: bookcloud → Users → Add user**
2. Set username, click **Save**
3. Go to the **Credentials** tab → set a password → toggle **Temporary** off

### Create a user via Admin REST API

```bash
# Get admin token
ADMIN_TOKEN=$(curl -s -X POST http://localhost:8080/realms/master/protocol/openid-connect/token \
  -d "grant_type=password" \
  -d "client_id=admin-cli" \
  -d "username=admin" \
  -d "password=<admin-password>" | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

# Create user
curl -s -X POST http://localhost:8080/admin/realms/bookcloud/users \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "newuser",
    "enabled": true,
    "credentials": [{"type": "password", "value": "newpass", "temporary": false}]
  }'
```

---

## Roles

Roles are defined at the realm level and embedded in the JWT under `realm_access.roles`.

| Role | Intended use |
|---|---|
| `default-roles-bookcloud` | Assigned to all users automatically |
| `offline_access` | Allows refresh tokens to outlive the session |
| `uma_authorization` | Required for UMA (User-Managed Access) flows |
| `admin` | _(to be created)_ Full CRUD access to all services |
| `user` | _(to be created)_ Read and write access |
| `readonly` | _(to be created)_ Read-only access |

Role enforcement at the route level is handled by Kong's `acl` plugin combined with role claims extracted from the JWT.

---

## Troubleshooting

### `401 Unauthorized — No mandatory 'iss' claim`
The token's `iss` does not match the `key` in the Kong JWT consumer. Verify the issuer URL used during token issuance matches exactly what is configured in `kong/configmap.yaml`.

### `401 Unauthorized — Invalid signature`
The public key in the Kong consumer does not match the key Keycloak used to sign the token. Re-fetch the PEM key from `/realms/bookcloud` and update the configmap, then restart Kong.

### Token expired (`exp` claim in the past)
Access tokens expire after 300 seconds. Re-obtain a fresh token using the curl command above.

### Keycloak pod not ready
```bash
kubectl -n bookcloud describe pod -l app=keycloak
kubectl -n bookcloud logs deployment/keycloak
```

Check that the Postgres database (if used) is healthy and the `keycloak-credentials` secret is correctly mounted.

---

## Useful Commands

```bash
# Check Keycloak pod status
kubectl -n bookcloud get pods -l app=keycloak

# View Keycloak logs
kubectl -n bookcloud logs deployment/keycloak -f

# Port-forward Keycloak admin console
kubectl -n bookcloud port-forward service/keycloak 8080:80

# Decode a JWT token (without signature verification)
echo "<access_token>" | python3 -c "
import sys, base64, json
token = sys.stdin.read().strip()
payload = token.split('.')[1]
payload += '=' * (4 - len(payload) % 4)
print(json.dumps(json.loads(base64.b64decode(payload)), indent=2))
"

# List all users in the bookcloud realm
curl -s http://localhost:8080/admin/realms/bookcloud/users \
  -H "Authorization: Bearer $ADMIN_TOKEN" | python3 -m json.tool
```
