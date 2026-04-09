FROM rust:1.75-bookworm AS builder

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
        bash \
        ca-certificates \
        curl \
        docker.io \
        git \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash orbit

WORKDIR /workspace

COPY --from=builder /src/target/release/orbit-server /usr/local/bin/orbit-server

RUN mkdir -p /workspace /var/lib/orbit/server /var/lib/orbit/agents \
    && chown -R orbit:orbit /workspace /var/lib/orbit

USER orbit

EXPOSE 8788

CMD ["orbit-server"]
