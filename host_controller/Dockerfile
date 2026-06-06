# ─── Stage 1: build ──────────────────────────────────────────────────────────
# Pinned to bookworm so the binary links against the same glibc as the
# debian:bookworm-slim runtime (plain rust:1-slim now tracks trixie/glibc 2.39).
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
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
RUN cat Cargo-test.toml >> Cargo.toml

COPY tests/ tests/
COPY features/ features/

# Docker `COPY` preserves source mtimes, which can predate the cached build
# artifacts and make cargo treat the real test sources as "fresh" — silently
# running nothing. Touch every test source so it is always recompiled here.
RUN find tests -type f -name '*.rs' -exec touch {} + ; cargo test --no-run

# ─── Stage 3: mutants ────────────────────────────────────────────────────────
FROM test AS mutants

RUN cargo install --locked cargo-mutants@26.0.0
COPY config/mutants.toml config/mutants-integration.toml config/

# ─── Stage 4: runtime ────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# openssh-client: the controller shells out to `ssh` to reach the parent host.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 openssh-client \
    && rm -rf /var/lib/apt/lists/*

# Runs as root inside the container so it can read the mounted SSH key, which is
# owned by the deploying host user and kept at mode 0600. This does NOT weaken
# the "non-root" guarantee: commands run on the host as botuser (no sudo), so the
# container's own uid is irrelevant to host-side privilege.

COPY --from=builder /build/target/release/host_controller /usr/local/bin/host_controller

ENTRYPOINT ["/usr/local/bin/host_controller"]
CMD ["bot"]
