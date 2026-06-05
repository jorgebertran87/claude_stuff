# ─── Stage 1: build ──────────────────────────────────────────────────────────
FROM rust:1-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies separately from application code.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && touch src/lib.rs \
    && cargo build --release \
    && rm -rf src

# Build the real source.
COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

# ─── Stage 2: test ───────────────────────────────────────────────────────────
FROM builder AS test

# Merge the test-specific Cargo entries (dev-dependencies + [[test]] targets).
COPY config/Cargo-test.toml ./
RUN cat Cargo-test.toml >> Cargo.toml \
    && mkdir -p tests/unit tests/integration \
    && printf 'fn main() {}\n' | tee \
         tests/unit/service_manager.rs \
         tests/integration/alias_registry.rs \
         tests/integration/docker_control.rs > /dev/null \
    && cargo test --no-run \
    && rm -rf tests

# Copy test files and feature files.
COPY tests/ tests/
COPY features/ features/

# Docker `COPY` preserves the source mtimes, which can be older than the stub
# test artifacts compiled above — that would make `cargo test` consider the
# real tests "fresh" and silently run the stubs. Bump the mtimes so the real
# test sources are always recompiled at run time.
RUN touch tests/unit/*.rs tests/integration/*.rs

# ─── Stage 3: mutants ────────────────────────────────────────────────────────
FROM test AS mutants

RUN cargo install --locked cargo-mutants@26.0.0

COPY config/mutants.toml config/mutants-integration.toml config/

# ─── Stage 4: runtime ────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Directory for the alias config — mount your aliases.yaml here.
RUN mkdir -p /config

COPY --from=builder /build/target/release/services_controller /usr/local/bin/services_controller

# Run as a non-root user.
RUN useradd -r -M -s /bin/false appuser
USER appuser

ENTRYPOINT ["/usr/local/bin/services_controller"]
