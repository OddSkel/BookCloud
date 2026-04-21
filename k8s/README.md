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
3. Enables the `metrics-server` addon required by Horizontal Pod Autoscalers.
4. Builds the local images used by the deployments:
   - `bookcloud/api-gateway:latest`
   - `bookcloud/book-catalog:latest`
   - `bookcloud/author-catalog:latest`
   - `bookcloud/rating-catalog:latest`
   - `bookcloud/compare-service:latest`
   - `bookcloud/genre-analysis-service:latest`
5. Applies all manifests with `kubectl apply -k k8s`.
6. Waits for the deployments to become available.

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
minikube -p bookcloud addons enable metrics-server
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
for deployment in $(kubectl -n bookcloud get deployments -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}'); do
  kubectl -n bookcloud rollout status "deployment/$deployment" --timeout=300s
done
kubectl -n bookcloud get pods,svc,ingress,hpa
```

## Deployment Features

The Kubernetes manifests include these operational features for all six application microservices:

- Deployment and Service YAML files.
- Environment-variable ConfigMaps.
- Resource requests and limits.
- Readiness, liveness, and startup probes.
- Explicit `RollingUpdate` deployment strategy.
- Horizontal Pod Autoscalers using CPU and memory utilization.

The database pods use PersistentVolumeClaims for PostgreSQL data persistence.

The HPA manifests require metrics from `metrics-server`. The startup script enables the Minikube addon automatically.

Current autoscaling policy:

- `api-gateway`: 2 to 5 replicas.
- Other application services: 1 to 4 replicas.
- CPU target: 70% average utilization.
- Memory target: 80% average utilization.

## GKE Deployment

For Google Kubernetes Engine, local Minikube images cannot be used directly. The GKE deployment script builds the service images, pushes them to Artifact Registry, renders a temporary copy of the manifests with the pushed image URLs, and applies that copy to the GKE cluster.

Requirements:

- Google Cloud SDK authenticated with `gcloud auth login`.
- Docker running locally.
- A Google Cloud project with billing enabled.
- IAM permissions to create or use GKE clusters and Artifact Registry repositories.

Deploy to GKE:

```bash
GCP_PROJECT_ID=<your-gcp-project-id> ./k8s/deploy_gke.sh
```

Default GKE settings:

- Artifact Registry region: `europe-west1`
- GKE zone: `europe-west1-b`
- Cluster name: `bookcloud-gke`
- Machine type: `e2-standard-2`
- Nodes: `2`
- Artifact Registry repository: `bookcloud`
- Ingress host: `api.bookcloud.local`

Override defaults:

```bash
GCP_PROJECT_ID=<project> \
GCP_REGION=europe-west1 \
GCP_ZONE=europe-west1-b \
GKE_CLUSTER_NAME=bookcloud-gke \
GKE_MACHINE_TYPE=e2-standard-2 \
GKE_NUM_NODES=2 \
BOOKCLOUD_INGRESS_HOST=api.bookcloud.example.com \
./k8s/deploy_gke.sh
```

If you already created the GKE cluster and only want to deploy to it:

```bash
GCP_PROJECT_ID=<project> GKE_CREATE_CLUSTER=0 ./k8s/deploy_gke.sh
```

After deployment, check the external IP:

```bash
kubectl -n bookcloud get ingress api-gateway
```

If you do not have DNS configured yet, test with `curl --resolve`:

```bash
curl --resolve api.bookcloud.local:80:<EXTERNAL_IP> http://api.bookcloud.local/health
```

For a real public hostname, create a DNS `A` record pointing your chosen host to the ingress external IP.

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

Rollback all application microservices to their previous rollout revision:

```bash
./k8s/rollback_service.sh
```

Check autoscalers:

```bash
kubectl -n bookcloud get hpa
kubectl -n bookcloud describe hpa api-gateway
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
