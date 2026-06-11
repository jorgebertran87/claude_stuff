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
         tests/unit/change_detector.rs \
         tests/unit/monitor_spawner.rs \
         tests/unit/strip_tags.rs \
         tests/integration/monitor_store.rs \
         tests/integration/flare_source.rs > /dev/null \
    && cargo test --no-run \
    && rm -rf tests

# Copy test files and feature files.
COPY tests/ tests/
COPY features/ features/

# Docker `COPY` preserves source mtimes, which can predate the stub-built test
# binaries above and make cargo treat the real test sources as "fresh" —
# silently running the empty stubs and faking green. Touch every test source so
# it is always recompiled here.
RUN find tests -type f -name '*.rs' -exec touch {} + ; cargo test --no-run

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

# Directory for state persistence — mount a named volume here.
RUN mkdir -p /data

COPY --from=builder /build/target/release/changes_detector /usr/local/bin/changes_detector

# Run as a non-root user.
RUN useradd -r -M -s /bin/false appuser && chown appuser /data
USER appuser

ENTRYPOINT ["/usr/local/bin/changes_detector"]
