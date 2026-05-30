# BookCloud GKE Deployment

## Kong Static IP

The CD workflow reserves a regional Google Cloud static IP for the Kong
`LoadBalancer` service before applying the Kubernetes manifests.

Configurable GitHub variables:

- `KONG_STATIC_IP_NAME`, defaults to `bookcloud-kong-ip`
- `KONG_STATIC_IP_REGION`, defaults to `GKE_LOCATION`

The workflow stores the resolved address in `KONG_STATIC_IP` and injects it into
the rendered Kong service manifest before `kubectl apply -k`.

Check the reserved address:

```bash
gcloud compute addresses list
kubectl -n bookcloud get svc kong -o wide
```

As long as the reserved address is preserved, recreating the cluster should not
change the public Kong IP.

## Cloud Teardown

Use the teardown script to remove the BookCloud cloud environment:

```bash
chmod +x scripts/cloud-down.sh
./scripts/cloud-down.sh
```

The script requires typing `DELETE` before it removes resources.

By default, the reserved Kong IP is preserved for the next deploy:

```bash
DELETE_STATIC_IP=false ./scripts/cloud-down.sh
```

To also delete the reserved IP:

```bash
DELETE_STATIC_IP=true ./scripts/cloud-down.sh
```

Useful defaults can be overridden with environment variables:

```bash
PROJECT_ID=cloud-computing-2526 \
GKE_CLUSTER=bookcloud-gke \
GKE_LOCATION=europe-west1 \
K8S_NAMESPACE=bookcloud \
./scripts/cloud-down.sh
```
