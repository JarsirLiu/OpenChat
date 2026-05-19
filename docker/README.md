# OpenChat Docker

This directory contains:

- `docker-compose.yml`: full production-style stack that pulls prebuilt app images
- `docker-compose.local.yml`: full stack for local image builds
- `docker-compose.middleware.yml`: local middleware only for host-run development

## Full Stack From Registry

```bash
cd docker
cp .env.example .env
docker compose pull app nginx
docker compose up -d
```

Set `OPENCHAT_APP_IMAGE` and `OPENCHAT_NGINX_IMAGE` in `docker/.env` when using your own registry images.

## Full Stack With Local Build

```bash
cd docker
cp .env.example .env
docker compose -f docker-compose.local.yml build app nginx
docker compose -f docker-compose.local.yml up -d
```

This stack starts:

- `nginx`: serves the built web app and proxies `/api`, `/api/stream`, `/health`
- `app`: OpenChat Rust API
- `acme-init`: initial Let's Encrypt certificate issuance
- `acme-renew`: periodic Let's Encrypt renewal
- `postgres`: PostgreSQL 15 for multi-user persistent data
- `minio`: S3-compatible object storage
- `minio-init`: bucket bootstrap and public-read policy

For production, set:

```env
OPENCHAT_S3_PUBLIC_BASE_URL=https://your-domain.com/openchat-media
OPENCHAT_PROVIDER_SECRET_KEY=change-this
OPENCHAT_SERVER_NAME=your-domain.com
SSL_CERT_NAME=your-domain.com
ENABLE_LETSENCRYPT=true
LETSENCRYPT_DOMAIN=your-domain.com
LETSENCRYPT_ALT_DOMAINS=www.your-domain.com
LETSENCRYPT_EMAIL=you@example.com
```

## Middleware Only

```bash
cd docker
cp .env.example .env
docker compose -f docker-compose.middleware.yml up -d
```

This stack starts only:

- `postgres`: PostgreSQL 15 for multi-user persistent data
- `minio`: S3-compatible object storage
- `minio-init`: bucket bootstrap and public-read policy

## Required App Env

If the OpenChat app runs on your host machine, point it at the middleware stack with:

```env
OPENCHAT_DATABASE_URL=postgresql://openchat:openchat123456@localhost:5432/openchat
OPENCHAT_MEDIA_STORAGE_BACKEND=s3
OPENCHAT_S3_ENDPOINT=http://localhost:9000
OPENCHAT_S3_REGION=us-east-1
OPENCHAT_S3_BUCKET=openchat-media
OPENCHAT_S3_ACCESS_KEY_ID=minioadmin
OPENCHAT_S3_SECRET_ACCESS_KEY=minioadmin
OPENCHAT_S3_PUBLIC_BASE_URL=http://localhost:9000/openchat-media
OPENCHAT_S3_FORCE_PATH_STYLE=true
```

If the app itself runs inside Docker on the same network as `minio`, use:

```env
OPENCHAT_DATABASE_URL=postgresql://openchat:openchat123456@postgres:5432/openchat
OPENCHAT_S3_ENDPOINT=http://minio:9000
OPENCHAT_S3_PUBLIC_BASE_URL=http://localhost:9000/openchat-media
```

## Tencent Registry

If you later push images to Tencent Cloud Registry, change these in `docker/.env`:

```env
OPENCHAT_APP_IMAGE=ccr.ccs.tencentyun.com/<namespace>/openchat-app:latest
OPENCHAT_NGINX_IMAGE=ccr.ccs.tencentyun.com/<namespace>/openchat-nginx:latest
```

Then build and push manually:

```bash
docker compose -f docker-compose.local.yml build app nginx
docker compose -f docker-compose.local.yml push app nginx
```

## Notes

- The app does not need a local file media server when running in S3 mode.
- The public base URL must be reachable by your browser.
- This stack uses MinIO as the S3-compatible backend.
- `nginx` disables proxy buffering for `/api/stream/*` so SSE chat streaming stays real-time.
- `nginx` also proxies `/openchat-media/*` to MinIO, so uploaded media can stay on the same public domain.
- If no real certificate exists yet, `nginx` bootstraps a short-lived self-signed cert so HTTPS can start before ACME finishes.
- ACME uses HTTP-01 under `/.well-known/acme-challenge/`, so ports `80` and `443` both need to be reachable from the public internet.
