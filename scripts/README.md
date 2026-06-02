# Scripts

## Dataset sample

Generate a consistent reduced dataset from `data/data_clean/normalized_out/`:

```bash
BOOKCLOUD_SAMPLE_SIZE=1000 python3 scripts/create_dataset_sample.py
```

The script writes the reduced CSV files to:

```text
data/data_clean/normalized_sample/
```

Upload the sample to Google Cloud Storage:

```bash
./scripts/upload_dataset_sample_to_gcs.sh
```

By default, the upload target is:

```text
gs://bookcloud-dataset/normalized_sample
```

Override the bucket with:

```bash
BOOKCLOUD_DATASET_BUCKET=my-bucket ./scripts/upload_dataset_sample_to_gcs.sh
```

Import the sample with the dataset import flow:

```bash
DATASET_GCS_URI=gs://bookcloud-dataset/normalized_sample \
DATASET_DIR=data/data_clean/normalized_sample \
./scripts/06-import-dataset.sh
```

For the current local Kubernetes importer in this repository, use:

```bash
BOOKCLOUD_CSV_DIR=data/data_clean/normalized_sample \
./k8s/import_data.sh
```
