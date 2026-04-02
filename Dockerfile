# Yellow Rock Memory — Multi-stage Docker Build
# Forensic communication archive daemon with bearer token auth
#
# Build:  docker build -t yellow-rock-memory .
# Run:    docker run -d -p 9077:9077 -e GRM_API_KEY=<key> -e GRM_PRINCIPAL_ID=<id> -v yrm-data:/data yellow-rock-memory

# ---- Stage 1: Build ----
FROM rust:1.82-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY benches/ benches/
COPY tests/ tests/

RUN cargo build --release --bin yellow-rock-memory && \
    strip target/release/yellow-rock-memory

# ---- Stage 2: Runtime ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* && \
    groupadd -r yrm && useradd -r -g yrm -d /data -s /sbin/nologin yrm && \
    mkdir -p /data && chown yrm:yrm /data

COPY --from=builder /build/target/release/yellow-rock-memory /usr/local/bin/yellow-rock-memory

USER yrm
WORKDIR /data

# Database persisted via volume mount at /data
ENV YELLOW_ROCK_MEMORY_DB=/data/yellow-rock-memory.db

# Default: HTTP server mode on 0.0.0.0:9077
# Override with "mcp" for MCP stdio mode
EXPOSE 9077

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["yellow-rock-memory", "--db", "/data/yellow-rock-memory.db", "stats"]

ENTRYPOINT ["yellow-rock-memory", "--db", "/data/yellow-rock-memory.db"]
CMD ["serve", "--host", "0.0.0.0", "--port", "9077"]
