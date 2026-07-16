FROM rust:1.88-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        libssl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p orbit-server

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash orbit

WORKDIR /workspace

COPY --from=builder /src/target/release/orbit-server /usr/local/bin/orbit-server

RUN mkdir -p /workspace/workspaces /var/lib/orbit/server /var/lib/orbit/agents \
    && chown -R orbit:orbit /workspace /var/lib/orbit

USER orbit

EXPOSE 8788

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://127.0.0.1:8788/health || exit 1

CMD ["orbit-server"]
