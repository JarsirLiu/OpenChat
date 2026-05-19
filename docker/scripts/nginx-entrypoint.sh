#!/bin/sh
set -eu

SSL_CERT_NAME="${SSL_CERT_NAME:-localhost}"
CERT_DIR="${NGINX_CERT_DIR:-/etc/nginx/ssl/${SSL_CERT_NAME}}"
FULLCHAIN_FILE="${CERT_DIR}/fullchain.pem"
PRIVKEY_FILE="${CERT_DIR}/privkey.pem"
BOOTSTRAP_MARKER="${CERT_DIR}/.bootstrap-self-signed"
CERT_SUBJECT="/C=CN/ST=Shanghai/L=Shanghai/O=OpenChat/OU=Temporary/CN=${SSL_CERT_NAME}"
INTERVAL_SECONDS="${NGINX_CERT_RELOAD_INTERVAL_SECONDS:-21600}"
CHECK_SECONDS="${NGINX_CERT_RELOAD_CHECK_SECONDS:-30}"
FLAG_FILE="${NGINX_CERT_RELOAD_FLAG_FILE:-/etc/nginx/ssl/.reload-nginx}"

mkdir -p "${CERT_DIR}"

if [ ! -s "${FULLCHAIN_FILE}" ] || [ ! -s "${PRIVKEY_FILE}" ]; then
  echo "[nginx] no certificate found, generating temporary self-signed certificate ..."
  tmpdir="$(mktemp -d)"
  openssl req -x509 -nodes -newkey rsa:2048 -days 2 \
    -keyout "${tmpdir}/privkey.pem" \
    -out "${tmpdir}/fullchain.pem" \
    -subj "${CERT_SUBJECT}" >/dev/null 2>&1
  cp "${tmpdir}/fullchain.pem" "${FULLCHAIN_FILE}"
  cp "${tmpdir}/privkey.pem" "${PRIVKEY_FILE}"
  touch "${BOOTSTRAP_MARKER}"
  rm -rf "${tmpdir}"
else
  rm -f "${BOOTSTRAP_MARKER}" || true
fi

envsubst '${OPENCHAT_SERVER_NAME} ${OPENCHAT_APP_UPSTREAM} ${OPENCHAT_MINIO_UPSTREAM} ${OPENCHAT_CLIENT_MAX_BODY_SIZE} ${SSL_CERT_NAME}' \
  < /etc/nginx/templates/default.conf.template \
  > /etc/nginx/conf.d/default.conf

(
  while true; do
    sleep "${INTERVAL_SECONDS}"
    echo "[nginx] periodic reload for certificate refresh"
    nginx -s reload || true
  done
) &

(
  while true; do
    sleep "${CHECK_SECONDS}"
    if [ -f "${FLAG_FILE}" ]; then
      echo "[nginx] detected certificate reload flag"
      rm -f "${FLAG_FILE}" || true
      nginx -s reload || true
    fi
  done
) &

exec nginx -g 'daemon off;'
