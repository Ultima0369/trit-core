# ── Stage 1: Build datacore-collect ──────────────────────────
FROM rust:1.83-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests (caching layer)
COPY Cargo.toml Cargo.lock ./
COPY dataforge/Cargo.toml ./dataforge/
COPY datacore/Cargo.toml ./datacore/
COPY aurora/Cargo.toml ./aurora/
COPY src-tauri/Cargo.toml ./src-tauri/

# Dummy source files to prefetch dependencies
RUN mkdir -p src dataforge/src datacore/src aurora/src src-tauri/src && \
    echo 'fn main() {}' > src/lib.rs && \
    echo 'fn main() {}' > dataforge/src/lib.rs && \
    echo 'fn main() {}' > datacore/src/lib.rs && \
    echo 'fn main() {}' > aurora/src/lib.rs && \
    echo 'fn main() {}' > src-tauri/src/lib.rs

# Pre-build dependencies (this layer is cached unless manifests change)
RUN cargo build --release --bin datacore-collect -p datacore 2>/dev/null; \
    echo "dependency prefetch done"

# Copy real source (invalidates cache only when source changes)
COPY src/ ./src/
COPY dataforge/src/ ./dataforge/src/
COPY datacore/src/ ./datacore/src/
COPY aurora/src/ ./aurora/src/

# Build the binary
RUN cargo build --release --bin datacore-collect -p datacore

# ── Stage 2: Runtime ─────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/datacore-collect /usr/local/bin/datacore-collect

RUN mkdir -p /data/cache
ENV XDG_CACHE_HOME=/data/cache

# Default: single run, compact JSON output
ENTRYPOINT ["/usr/local/bin/datacore-collect"]
CMD ["--compact"]
