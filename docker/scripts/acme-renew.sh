#!/bin/sh
set -eu

if [ "${ENABLE_LETSENCRYPT:-false}" != "true" ]; then
  echo "[acme-renew] ENABLE_LETSENCRYPT is not true, idle forever."
  tail -f /dev/null
fi

ACME_HOME="${ACME_HOME:-/acme-data}"
INTERVAL_SECONDS="${LETSENCRYPT_RENEW_INTERVAL_SECONDS:-43200}"

mkdir -p "${ACME_HOME}"

while true; do
  echo "[acme-renew] running acme.sh cron ..."
  acme.sh --home "${ACME_HOME}" --cron || true
  echo "[acme-renew] sleep ${INTERVAL_SECONDS}s"
  sleep "${INTERVAL_SECONDS}"
done
