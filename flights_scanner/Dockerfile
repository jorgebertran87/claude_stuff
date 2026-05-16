FROM rust:1.88-slim AS tester

WORKDIR /app

RUN rustup component add clippy

COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN cargo test 2>&1
RUN cargo clippy -- -D warnings

# ── Release build ─────────────────────────────────────────────────────────────

FROM rust:1.88-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

# ── Runtime image ─────────────────────────────────────────────────────────────

FROM debian:bookworm-slim AS runner

WORKDIR /app
COPY --from=builder /app/target/release/flights_scanner .
EXPOSE 3000
CMD ["./flights_scanner"]
