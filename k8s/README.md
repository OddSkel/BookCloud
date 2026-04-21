# BookCloud on Local Kubernetes

This directory contains the Kubernetes manifests required to run BookCloud locally in the `bookcloud` namespace.

The recommended local setup uses Minikube, because it provides a local Kubernetes cluster and lets you build images directly inside the environment used by the cluster.

## Requirements

- Docker
- Minikube
- kubectl

Check that the tools are installed:

```bash
docker --version
minikube version
kubectl version --client
```

## Automatic Startup

From the project root:

```bash
chmod +x k8s/startup_service.sh
./k8s/startup_service.sh
```

The script automatically:

1. Starts Minikube if it is not already running.
2. Enables the `ingress` addon.
3. Builds the local images used by the deployments:
   - `bookcloud/api-gateway:latest`
   - `bookcloud/book-catalog:latest`
   - `bookcloud/author-catalog:latest`
   - `bookcloud/rating-catalog:latest`
   - `bookcloud/compare-service:latest`
   - `bookcloud/genre-analysis-service:latest`
4. Applies all manifests with `kubectl apply -k k8s`.
5. Waits for the deployments to become available.

By default, Minikube creates a local cluster with one node. To request more nodes during the first cluster startup:

```bash
BOOKCLOUD_MINIKUBE_NODES=2 ./k8s/startup_service.sh
```

Note: if the Minikube profile already exists, the node count is not changed automatically. To recreate the cluster from scratch, delete the profile first with `minikube delete -p bookcloud`.

## Manual Startup

If you prefer to run the steps manually:

```bash
minikube start -p bookcloud
minikube -p bookcloud addons enable ingress
```

Build the images inside Minikube:

```bash
minikube -p bookcloud image build -t bookcloud/api-gateway:latest src/api-gateway
minikube -p bookcloud image build -t bookcloud/book-catalog:latest src/services/book-catalog
minikube -p bookcloud image build -t bookcloud/author-catalog:latest src/services/author-catalog
minikube -p bookcloud image build -t bookcloud/rating-catalog:latest src/services/rating-catalog
minikube -p bookcloud image build -t bookcloud/compare-service:latest src/services/compare-service
minikube -p bookcloud image build -t bookcloud/genre-analysis-service:latest src/services/genre-analysis-service
```

Apply all manifests:

```bash
kubectl apply -k k8s
kubectl -n bookcloud rollout status deployment --all --timeout=300s
kubectl -n bookcloud get pods,svc,ingress
```

## API Gateway Access

The ingress is configured for this host:

```text
api.bookcloud.local
```

Get the Minikube IP:

```bash
minikube -p bookcloud ip
```

Then add an entry to `/etc/hosts`, replacing `<MINIKUBE_IP>` with the returned IP:

```text
<MINIKUBE_IP> api.bookcloud.local
```

Example request:

```bash
curl http://api.bookcloud.local/health
```

If you do not want to edit `/etc/hosts`, use port-forwarding instead:

```bash
kubectl -n bookcloud port-forward svc/api-gateway 8080:80
curl http://localhost:8080/health
```

## Useful Commands

List all resources:

```bash
kubectl -n bookcloud get all
kubectl -n bookcloud get pvc
kubectl -n bookcloud get ingress
```

View service logs:

```bash
kubectl -n bookcloud logs deployment/api-gateway
kubectl -n bookcloud logs deployment/book-catalog
```

Restart a deployment after rebuilding an image:

```bash
kubectl -n bookcloud rollout restart deployment/api-gateway
```

Remove the application resources:

```bash
kubectl delete -k k8s
```

Stop the local cluster:

```bash
minikube stop -p bookcloud
```

## Import Data

After the Kubernetes services are running, import the normalized CSV files into the PostgreSQL databases:

```bash
./k8s/import_data.sh
```

The script opens temporary `kubectl port-forward` connections to:

- `book-db` -> `book_catalog`
- `author-db` -> `author_catalog`
- `rating-db` -> `rating_catalog`
- `genre-db` -> `genre_analysis`

By default, the script keeps existing rows and uses `ON CONFLICT DO NOTHING`.

To truncate the service tables before importing:

```bash
BOOKCLOUD_DROP_BEFORE_IMPORT=1 ./k8s/import_data.sh
```

Optional settings:

```bash
BOOKCLOUD_IMPORT_CHUNK_SIZE=5000 ./k8s/import_data.sh
BOOKCLOUD_CSV_DIR=/path/to/normalized_out ./k8s/import_data.sh
```
