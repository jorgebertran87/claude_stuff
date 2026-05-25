# ─── Stage 1: build ──────────────────────────────────────────────────────────
FROM rust:1-alpine AS builder

# musl-dev is required to link against musl libc on Alpine.
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /build

# Cache dependencies separately from application code.
# Copy manifests first so Docker reuses this layer as long as Cargo.toml/lock
# haven't changed.
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main so `cargo build --release` can resolve all dependencies.
RUN mkdir src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Now copy the real source and do the final build.
COPY src ./src
# Touch main.rs so Cargo knows it changed (otherwise it reuses the dummy binary).
RUN touch src/main.rs \
    && cargo build --release

# ─── Stage 2: runtime ────────────────────────────────────────────────────────
FROM alpine:3.20 AS runtime

# ca-certificates is needed for HTTPS calls to the Telegram API.
RUN apk add --no-cache ca-certificates tzdata

# Directory for state persistence — mount a named volume here.
RUN mkdir -p /data

COPY --from=builder /build/target/release/changes_detector /usr/local/bin/changes_detector

# Run as a non-root user.
RUN adduser -D -H appuser \
    && chown appuser /data
USER appuser

ENTRYPOINT ["/usr/local/bin/changes_detector"]
