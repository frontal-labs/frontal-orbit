# Multi-stage build for Orbit
FROM rust:1.75-bookworm as builder

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
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /workspace

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the application
RUN cargo build --release --workspace

# Runtime stage
FROM debian:bookworm-slim

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
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash orbit

# Set working directory
WORKDIR /workspace

# Copy the binary from builder stage
COPY --from=builder /workspace/target/release/orbit /usr/local/bin/orbit

# Set permissions
RUN chmod +x /usr/local/bin/orbit

# Create necessary directories
RUN mkdir -p /workspace/.orbit /workspace/.sandbox-home \
    && chown -R orbit:orbit /workspace

# Switch to non-root user
USER orbit

# Environment variables
ENV CARGO_TERM_COLOR=always
ENV ORBIT_HOME=/workspace/.orbit
ENV SANDBOX_HOME=/workspace/.sandbox-home

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD orbit doctor || exit 1

# Expose port (if needed for web interface)
EXPOSE 8080

# Default command
CMD ["orbit"]
