# SPDX-License-Identifier: Apache-2.0
# Copyright 2017-2026 Pecu Novus Network / MegaHoot Technologies
#
# Multi-stage Dockerfile for pecu-novus
# Stage 1 (builder): compiles the release binary with full Rust toolchain
# Stage 2 (runtime): copies only the binary into a minimal Debian slim image
#
# Final image size: ~80MB vs ~2GB for a full Rust build image

# ─── Stage 1: Builder ────────────────────────────────────────────────────────
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /build

# Install system deps required by sled (needs libgcc) and linking
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies separately from source code.
# Copy manifests first — Docker layer cache means this only re-runs
# when Cargo.toml or Cargo.lock changes, not on every source edit.
COPY Cargo.toml Cargo.lock ./

# Create a dummy main so `cargo build` can resolve and cache all deps
RUN mkdir -p src && \
    echo 'fn main() {}' > src/main.rs && \
    echo 'pub fn dummy() {}' > src/lib.rs && \
    cargo build --release 2>&1 && \
    rm -rf src

# Now copy the real source and build for real
COPY src ./src
COPY tests ./tests

# Touch main.rs so Cargo knows to recompile (dummy was already compiled)
RUN touch src/main.rs src/lib.rs && \
    cargo build --release && \
    strip target/release/pecu-node

# ─── Stage 2: Runtime ────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="Pecu Novus Node"
LABEL org.opencontainers.image.description="Pecu Novus Blockchain — Rust Implementation (Pecu 3.0 Themis)"
LABEL org.opencontainers.image.url="https://pecunovus.com"
LABEL org.opencontainers.image.source="https://github.com/MegaHoot/pecunovus"
LABEL org.opencontainers.image.vendor="MegaHoot Technologies"
LABEL org.opencontainers.image.licenses="Apache-2.0"

# Minimal runtime deps: ca-certificates for TLS, libgcc for sled
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libgcc-s1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin pecu

# Copy binary from builder
COPY --from=builder /build/target/release/pecu-node /usr/local/bin/pecu-node

# Data directory for sled persistent storage
RUN mkdir -p /data/pecu && chown pecu:pecu /data/pecu

# Run as non-root
USER pecu

# JSON-RPC port
EXPOSE 8545

# Persistent blockchain data
VOLUME ["/data/pecu"]

# Health check — hits eth_blockNumber every 30s
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf -X POST http://localhost:${PECU_RPC_PORT:-8545} \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
        | grep -q '"result"' || exit 1

ENV PECU_RPC_PORT=8545
ENV PECU_DATA_DIR=/data/pecu
ENV RUST_LOG=info

ENTRYPOINT ["pecu-node"]