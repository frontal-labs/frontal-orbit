# Multi-stage build for Orbit
FROM rust:1.82-bookworm@sha256:87f3b2f93b82995443a1a558c234212dafe79cfdc3af956539610560369ddcd0 as builder

# Install build dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        libssl-dev \
        pkg-config \
        sqlite3 \
        libsqlite3-dev \
        libpq-dev \
        curl \
        build-essential \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /workspace

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source files for caching
RUN mkdir -p crates/cli/src crates/server/src crates/runtime/src \
    && echo "fn main() {}" > crates/cli/src/main.rs \
    && echo "fn main() {}" > crates/server/src/main.rs \
    && echo "pub mod config;" > crates/runtime/src/lib.rs

# Build dependencies
RUN cargo build --release --workspace
RUN rm -rf crates/*/src

# Copy actual source code
COPY crates/ ./crates/

# Build the application with all targets
RUN cargo build --release --workspace --bins

# Runtime stage
FROM debian:bookworm-slim@sha256:4724b8cc51e33e398f0e2e15e18d5ec2851ff0c2280647e1310bc1642182655d

# Install runtime dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        sqlite3 \
        libpq5 \
        libssl3 \
        curl \
        postgresql-client \
        ca-certificates \
        && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash orbit

# Set working directory
WORKDIR /workspace

# Copy binaries from builder stage
COPY --from=builder /workspace/target/release/orbit /usr/local/bin/orbit
COPY --from=builder /workspace/target/release/mock-anthropic-service /usr/local/bin/mock-anthropic-service

# Set permissions
RUN chmod +x /usr/local/bin/orbit /usr/local/bin/mock-anthropic-service

# Create necessary directories
RUN mkdir -p /workspace/.orbit /workspace/.sandbox-home /workspace/data \
    && chown -R orbit:orbit /workspace

# Switch to non-root user
USER orbit

# Environment variables
ENV CARGO_TERM_COLOR=always
ENV ORBIT_HOME=/workspace/.orbit
ENV SANDBOX_HOME=/workspace/.sandbox-home
ENV ORBIT_SERVER_HOST=0.0.0.0
ENV ORBIT_SERVER_PORT=8788

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD orbit doctor || exit 1

# Expose ports
EXPOSE 8788 8080

# Default command
CMD ["orbit", "--help"]
