#!/bin/sh
set -eu

if [ "${ENABLE_LETSENCRYPT:-false}" != "true" ]; then
  echo "[acme-init] ENABLE_LETSENCRYPT is not true, skip."
  exit 0
fi

DOMAIN="${LETSENCRYPT_DOMAIN:-}"
if [ -z "${DOMAIN}" ]; then
  echo "[acme-init] LETSENCRYPT_DOMAIN is required."
  exit 1
fi

ACME_HOME="${ACME_HOME:-/acme-data}"
CERT_ROOT="${LETSENCRYPT_CERT_PATH:-/acme-certs}"
CA_SERVER="${LETSENCRYPT_CA:-letsencrypt}"
EMAIL="${LETSENCRYPT_EMAIL:-}"
ECC_CURVE="${LETSENCRYPT_ECC_CURVE:-ec-256}"
WEBROOT="${LETSENCRYPT_WEBROOT:-/var/www/acme}"
CERT_DIR="${CERT_ROOT}/${DOMAIN}"
BOOTSTRAP_MARKER="${CERT_DIR}/.bootstrap-self-signed"
ISSUED_MARKER="${CERT_DIR}/.letsencrypt-issued"
RETRY_SECONDS="${LETSENCRYPT_INIT_RETRY_SECONDS:-30}"

mkdir -p "${ACME_HOME}" "${CERT_DIR}" "${WEBROOT}"

if [ -f "${ISSUED_MARKER}" ] && [ -s "${CERT_DIR}/fullchain.pem" ] && [ -s "${CERT_DIR}/privkey.pem" ] && [ ! -f "${BOOTSTRAP_MARKER}" ]; then
  echo "[acme-init] certificate already exists at ${CERT_DIR}, skip."
  exit 0
fi

if [ -n "${EMAIL}" ]; then
  acme.sh --home "${ACME_HOME}" --register-account -m "${EMAIL}" --server "${CA_SERVER}" || true
fi

DOMAIN_ARGS="-d ${DOMAIN}"
if [ -n "${LETSENCRYPT_ALT_DOMAINS:-}" ]; then
  OLDIFS="$IFS"
  IFS=','
  for d in ${LETSENCRYPT_ALT_DOMAINS}; do
    d_trim=$(echo "$d" | tr -d '[:space:]')
    if [ -n "$d_trim" ]; then
      DOMAIN_ARGS="${DOMAIN_ARGS} -d ${d_trim}"
    fi
  done
  IFS="$OLDIFS"
fi

while true; do
  echo "[acme-init] issuing certificate for ${DOMAIN} ..."
  # shellcheck disable=SC2086
  if acme.sh --home "${ACME_HOME}" --server "${CA_SERVER}" --issue -w "${WEBROOT}" ${DOMAIN_ARGS} --keylength "${ECC_CURVE}"; then
    break
  fi

  echo "[acme-init] issue failed, retry in ${RETRY_SECONDS}s"
  sleep "${RETRY_SECONDS}"
done

echo "[acme-init] installing certificate into ${CERT_DIR} ..."
acme.sh --home "${ACME_HOME}" --server "${CA_SERVER}" --install-cert -d "${DOMAIN}" --ecc \
  --fullchain-file "${CERT_DIR}/fullchain.pem" \
  --key-file "${CERT_DIR}/privkey.pem" \
  --reloadcmd "touch ${CERT_ROOT}/.reload-nginx"

rm -f "${BOOTSTRAP_MARKER}" || true
touch "${ISSUED_MARKER}"

echo "[acme-init] done."
