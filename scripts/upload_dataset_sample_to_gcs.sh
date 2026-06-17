#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BUCKET="${BOOKCLOUD_DATASET_BUCKET:-bookcloud-dataset}"
LOCAL_DIR="$REPO_ROOT/data/data_clean/normalized_sample"
DEST_URI="gs://${BUCKET}/normalized_sample"

if ! command -v gcloud >/dev/null 2>&1; then
  echo "Error: gcloud command not found." >&2
  exit 1
fi

if [[ ! -d "$LOCAL_DIR" ]]; then
  echo "Error: local sample directory not found: $LOCAL_DIR" >&2
  echo "Run scripts/create_dataset_sample.py first." >&2
  exit 1
fi

shopt -s nullglob
CSV_FILES=("$LOCAL_DIR"/*.csv)
if [[ ${#CSV_FILES[@]} -eq 0 ]]; then
  echo "Error: no CSV files found in: $LOCAL_DIR" >&2
  exit 1
fi

echo "Uploading dataset sample to $DEST_URI ..."
gcloud storage cp -r "${CSV_FILES[@]}" "$DEST_URI/"
echo "Upload finished: $DEST_URI"
