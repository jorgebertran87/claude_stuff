# ─── Stage 1: build ──────────────────────────────────────────────────────────
# Pinned to bookworm so the builder's glibc matches the bookworm-slim runtime.
FROM rust:1-slim-bookworm AS builder

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
         tests/unit/price_comparison.rs \
         tests/unit/basket_reply.rs \
         tests/integration/mercadona_source.rs \
         tests/integration/dia_source.rs \
         tests/integration/lidl_source.rs \
         tests/integration/telegram_bot.rs > /dev/null \
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
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/prices_comparer /usr/local/bin/prices_comparer

# Run as a non-root user.
RUN useradd -r -M -s /bin/false appuser
USER appuser

ENTRYPOINT ["/usr/local/bin/prices_comparer"]
