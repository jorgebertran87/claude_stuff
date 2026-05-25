# ─── Stage 1: build ──────────────────────────────────────────────────────────
FROM rust:1-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies separately from application code.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build the real source.
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ─── Stage 2: runtime ────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Directory for state persistence — mount a named volume here.
RUN mkdir -p /data

COPY --from=builder /build/target/release/changes_detector /usr/local/bin/changes_detector

# Run as a non-root user.
RUN useradd -r -M -s /bin/false appuser && chown appuser /data
USER appuser

ENTRYPOINT ["/usr/local/bin/changes_detector"]
