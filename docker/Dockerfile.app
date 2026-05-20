ARG RUST_BUILDER_IMAGE=rust:1-slim-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim
ARG APT_MIRROR=http://deb.debian.org/debian
ARG APT_SECURITY_MIRROR=http://deb.debian.org/debian-security

FROM ${RUST_BUILDER_IMAGE} AS builder

ARG APT_MIRROR
ARG APT_SECURITY_MIRROR

WORKDIR /app

RUN sed -i "s|http://deb.debian.org/debian-security|${APT_SECURITY_MIRROR}|g; s|http://deb.debian.org/debian|${APT_MIRROR}|g" /etc/apt/sources.list.d/debian.sources \
    && apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first so dependency resolution can be cached.
COPY Cargo.toml Cargo.lock ./
COPY apps/api-rs/Cargo.toml apps/api-rs/Cargo.toml
COPY apps/worker-rs/Cargo.toml apps/worker-rs/Cargo.toml
COPY crates/security-core/Cargo.toml crates/security-core/Cargo.toml
COPY crates/account-core/Cargo.toml crates/account-core/Cargo.toml
COPY crates/catalog-core/Cargo.toml crates/catalog-core/Cargo.toml
COPY crates/infra/Cargo.toml crates/infra/Cargo.toml
COPY crates/openchat-core/Cargo.toml crates/openchat-core/Cargo.toml

# Create minimal sources so Cargo can fetch and cache dependencies before full source copy.
RUN mkdir -p apps/api-rs/src apps/worker-rs/src crates/security-core/src crates/account-core/src crates/catalog-core/src crates/infra/src crates/openchat-core/src \
    && printf 'fn main() {}\n' > apps/api-rs/src/main.rs \
    && printf 'fn main() {}\n' > apps/worker-rs/src/main.rs \
    && : > crates/security-core/src/lib.rs \
    && : > crates/account-core/src/lib.rs \
    && : > crates/catalog-core/src/lib.rs \
    && : > crates/infra/src/lib.rs \
    && : > crates/openchat-core/src/lib.rs

RUN cargo fetch --locked

# Copy the real sources after dependencies are cached.
COPY apps ./apps
COPY crates ./crates
COPY config ./config

RUN cargo build --release -p openchat-api

FROM ${RUNTIME_IMAGE}

ARG APT_MIRROR
ARG APT_SECURITY_MIRROR

WORKDIR /app

RUN sed -i "s|http://deb.debian.org/debian-security|${APT_SECURITY_MIRROR}|g; s|http://deb.debian.org/debian|${APT_MIRROR}|g" /etc/apt/sources.list.d/debian.sources \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/openchat-api /app/openchat-api
COPY --from=builder /app/config /app/config

ENV OPENCHAT_SERVER_ADDR=0.0.0.0:8787
ENV OPENCHAT_CATALOG_PATH=/app/config/model-catalog.json

EXPOSE 8787

CMD ["/app/openchat-api"]
