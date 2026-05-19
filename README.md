# OpenChat

An event-driven chat application scaffold with a Rust backend, a React frontend, and a reusable chat UI package.

## Architecture

- `apps/web`: the product web app
- `apps/api-rs`: OpenChat Rust API entrypoint
- `apps/worker-rs`: OpenChat Rust worker entrypoint for scheduling, push, and async jobs
- `crates/server-core`: shared Rust application core, protocol adapters, runtime glue, and infrastructure
- `packages/protocol`: normalized chat event contracts for the frontend
- `packages/chat-core`: chat runtime state machine
- `packages/ui`: reusable chat UI building blocks

## Commands

```bash
pnpm install
pnpm dev
pnpm build
```

`pnpm dev` starts both:

- `OpenChat` Rust BFF
- the `OpenChat` Vite frontend

Useful Rust commands:

```bash
pnpm check:server
pnpm dev:worker
```

## Docker

OpenChat includes two Docker modes:

- full stack: `docker/docker-compose.yml`
- middleware only: `docker/docker-compose.middleware.yml`

The full stack runs `nginx + app + postgres + minio`. The middleware stack is useful when you want to run the Rust app directly on the host.

## Local Middleware

OpenChat uses PostgreSQL plus S3-compatible object storage in development.

Start the middleware stack first:

```bash
cd docker
cp .env.example .env
docker compose -f docker-compose.middleware.yml up -d
```

Then start the app as usual. The root `.env` is already configured to point at:

- local PostgreSQL: `postgresql://openchat:openchat123456@localhost:5432/openchat`
- local MinIO: `http://localhost:9000`

If you want to inspect the bucket, open the MinIO console at `http://localhost:9001`.
