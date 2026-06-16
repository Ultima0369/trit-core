# Trit-Core Node Docker Image
# Multi-stage build for small final image size

FROM rust:1.84-slim-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY benches/ benches/

RUN cargo build --release --bin trit-node && \
    strip target/release/trit-node

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/trit-node /usr/local/bin/trit-node

# Default: start in Sovereign state, ready to receive resonance requests
ENTRYPOINT ["trit-node"]
CMD ["--frame", "Science", "--phase", "0.5"]
