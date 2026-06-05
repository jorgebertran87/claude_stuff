# ─── Stage 1: build ──────────────────────────────────────────────────────────
# Pinned to bookworm so the binary links against the same glibc as the
# debian:bookworm-slim runtime (plain rust:1-slim now tracks trixie/glibc 2.39).
FROM rust:1-slim-bookworm AS builder

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
         tests/unit/telegram_command.rs \
         tests/unit/compose_control.rs \
         tests/integration/alias_registry.rs > /dev/null \
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

# Docker CLI + compose plugin so the controller can run `docker compose`
# against the host daemon (its socket is mounted at runtime).
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl gnupg libssl3 \
    && install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] \
https://download.docker.com/linux/debian bookworm stable" > /etc/apt/sources.list.d/docker.list \
    && apt-get update && apt-get install -y --no-install-recommends \
        docker-ce-cli docker-compose-plugin \
    && rm -rf /var/lib/apt/lists/*

# Directory for the alias config — mount your aliases.yaml here.
RUN mkdir -p /config

COPY --from=builder /build/target/release/services_controller /usr/local/bin/services_controller

# Runs as root: driving the host Docker socket requires access to it.
ENTRYPOINT ["/usr/local/bin/services_controller"]
