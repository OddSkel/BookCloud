# `startup_service.sh` — BookCloud Local Bootstrap

This script automates the full local deployment of **BookCloud** on Minikube. It handles cluster setup, Docker image builds, Kubernetes manifest deployment, monitoring stack installation, Keycloak realm configuration, and initial user provisioning.

## Usage

```bash
# From the k8s/ directory:
dos2unix startup_service.sh
chmod +x startup_service.sh
./startup_service.sh
```

---

## Configuration

All parameters can be overridden via environment variables before running the script:

| Variable | Default | Description |
|----------|---------|-------------|
| `BOOKCLOUD_MINIKUBE_PROFILE` | `bookcloud` | Minikube profile name |
| `BOOKCLOUD_NAMESPACE` | `bookcloud` | Kubernetes namespace |
| `BOOKCLOUD_MINIKUBE_NODES` | `1` | Number of Minikube nodes |
| `BOOKCLOUD_ROLLOUT_TIMEOUT` | `300s` | Max wait time per deployment rollout |
| `BOOKCLOUD_MINIKUBE_MEMORY` | `6144` | Memory allocated to Minikube (MB) |
| `BOOKCLOUD_MINIKUBE_CPUS` | `4` | CPUs allocated to Minikube |

**Example — override memory and CPUs:**
```bash
BOOKCLOUD_MINIKUBE_MEMORY=8192 BOOKCLOUD_MINIKUBE_CPUS=6 ./startup_service.sh
```

---

## What It Does (Step by Step)

1. **Validates prerequisites** — exits early if any required command is missing.
2. **Starts Minikube** — skips if the profile is already running.
3. **Waits for the API server** — polls `kubectl cluster-info` until ready.
4. **Enables `metrics-server` addon** — required for HPA (Horizontal Pod Autoscaler).
5. **Builds all service images** inside Minikube's Docker daemon:
   - `bookcloud/api-gateway`
   - `bookcloud/book-catalog`
   - `bookcloud/author-catalog`
   - `bookcloud/rating-catalog`
   - `bookcloud/compare-service`
   - `bookcloud/genre-analysis-service`
6. **Installs the monitoring stack** (Prometheus + Grafana via `kube-prometheus-stack` Helm chart) into the `monitoring` namespace.
7. **Regenerates the Keycloak realm ConfigMap** from `keycloak/bookcloud-realm.json` — ensures the ConfigMap stays in sync with the realm file on disk.
8. **Applies all Kubernetes manifests** via `kubectl apply -k` (Kustomize).
9. **Waits for all deployments to roll out** successfully within the configured timeout.
10. **Port-forwards Keycloak** (`svc/keycloak → localhost:8090`) for local bootstrap access.
11. **Provisions Keycloak users** — creates the following users in the `bookcloud` realm and assigns their roles:

| Username | Role | Password |
|----------|------|---------|
| `testuser` | `user` | `password` |
| `adminuser` | `admin` | `password` |
| `readonlyuser` | `readonly` | `password` |

12. **Kills the port-forward** process after user provisioning completes.
13. **Prints a summary** of all pods, services, and HPAs in the namespace.

---

## Keycloak Realm

The realm definition lives at:

```
k8s/keycloak/bookcloud-realm.json
```

This file is automatically imported as a Kubernetes ConfigMap (`keycloak-realm`) on every script run, so changes to the JSON are always reflected without manual ConfigMap updates.

The Keycloak admin console is accessible during bootstrap at:

```
http://localhost:8090
Admin user: admin
Admin password: bookcloud-pass
```

> ⚠️ The port-forward is only active while the script is running. After it exits, use `kubectl -n bookcloud port-forward svc/keycloak 8090:8080` to reconnect.

---

## Monitoring

The Prometheus + Grafana stack is deployed to the `monitoring` namespace using values from:

```
k8s/monitoring/values-local.yaml
```

Helm is automatically installed if not already present on the system.

---

## Troubleshooting

**Image build fails:**
Check the build log for `ERROR: failed to build` or `error: could not compile`. Ensure Docker is running and that the source directory for the failing service is intact.

**Deployment rollout times out:**
Increase the timeout:
```bash
BOOKCLOUD_ROLLOUT_TIMEOUT=600s ./startup_service.sh
```

**Keycloak user provisioning fails:**
The script waits for `http://localhost:8090/realms/master` to respond before provisioning. If Keycloak is slow to start, it will wait. If it fails, re-run user setup manually using the Keycloak Admin REST API or the admin console.

**Minikube already running with wrong resources:**
Delete the profile and restart:
```bash
minikube delete -p bookcloud
./startup_service.sh
```
