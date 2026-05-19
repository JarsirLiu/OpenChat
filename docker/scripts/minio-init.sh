#!/bin/sh
set -eu

mc alias set openchat http://minio:9000 "${OPENCHAT_S3_ACCESS_KEY_ID}" "${OPENCHAT_S3_SECRET_ACCESS_KEY}"
mc mb --ignore-existing "openchat/${OPENCHAT_S3_BUCKET}"
mc anonymous set download "openchat/${OPENCHAT_S3_BUCKET}"
