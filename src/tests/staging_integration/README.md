# BookCloud Staging Tests

These tests run against a deployed staging environment through the public HTTP API.

## Run

```bash
BASE_URL=https://staging.example.com ./tests/staging_integration/run.sh
```

Optional variables:

- `API_BASE_URL`: overrides the default `${BASE_URL}/api`.
- `DATASET_DIR`: overrides `src/tests/dataset`.
- `BOOKCLOUD_ACCESS_TOKEN` or `AUTH_TOKEN`: bearer token for protected staging endpoints.

The tests are read-only. They intentionally avoid `POST`, `PUT`, and `DELETE` calls so repeated CD runs do not mutate staging data.
