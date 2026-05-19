This directory stores TLS certificates for the Docker nginx service.

- `SSL_CERT_NAME/fullchain.pem`
- `SSL_CERT_NAME/privkey.pem`

When `ENABLE_LETSENCRYPT=true`, the `acme-init` and `acme-renew` containers
manage these files automatically through ACME HTTP-01 challenges.

If no real certificate exists yet, the nginx entrypoint generates a temporary
self-signed certificate so HTTPS can start before ACME finishes.
