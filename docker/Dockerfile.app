ARG RUST_BUILDER_IMAGE=rust:1-slim-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

FROM ${RUST_BUILDER_IMAGE} AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY crates ./crates
COPY config ./config

RUN cargo build --release -p openchat-api

FROM ${RUNTIME_IMAGE}

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/openchat-api /app/openchat-api
COPY --from=builder /app/config /app/config

ENV OPENCHAT_SERVER_ADDR=0.0.0.0:8787
ENV OPENCHAT_CATALOG_PATH=/app/config/model-catalog.json

EXPOSE 8787

CMD ["/app/openchat-api"]
