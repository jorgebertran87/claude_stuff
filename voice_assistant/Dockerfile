# ── Stage 1: builder ──────────────────────────────────────────────────────────
FROM rust:1.86-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        portaudio19-dev \
        gcc \
        pkg-config \
        libpulse-dev \
        libasound2-dev \
        libsdl2-dev \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

ENV RUST_MIN_STACK=16777216

# Compile dependencies first — cached separately from application sources
COPY Cargo.toml Cargo.lock* build.rs ./
RUN mkdir -p src \
    && touch src/lib.rs \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src \
              target/release/.fingerprint/voice_assistant-* \
              target/release/deps/voice_assistant-* \
              target/release/deps/libvoice_assistant-* \
              target/release/voice_assistant

# Compile application sources
COPY src/ src/
RUN cargo build --release

# ── Stage 2: node-installer (official Node image + claude-code) ───────────────
FROM node:20-slim AS node-installer

RUN npm install -g @anthropic-ai/claude-code

# ── Stage 3: test-base (builder + Node/claude copied in + merged Cargo manifest)
FROM builder AS test-base

COPY --from=node-installer /usr/local/bin/node        /usr/local/bin/
COPY --from=node-installer /usr/local/bin/npm         /usr/local/bin/
COPY --from=node-installer /usr/local/bin/npx         /usr/local/bin/
COPY --from=node-installer /usr/local/bin/claude      /usr/local/bin/
COPY --from=node-installer /usr/local/lib/node_modules /usr/local/lib/node_modules

COPY config/Cargo-test.toml ./
RUN cat Cargo-test.toml >> Cargo.toml

# ── Stage 4: test (test-base + test files) ────────────────────────────────────
FROM test-base AS test

COPY tests/unit/ tests/unit/
COPY tests/integration/ tests/integration/
COPY features/ features/
COPY tests/files/ tests/files/

# ── Stage 5: mutants (test-base + cargo-mutants + .cargo/ + test files) ───────
FROM test-base AS mutants

RUN cargo install --locked cargo-mutants@26.0.0

COPY config/mutants.toml config/mutants-integration.toml config/
COPY tests/unit/ tests/unit/
COPY tests/integration/ tests/integration/
COPY features/ features/
COPY tests/files/ tests/files/

# ── Stage 6: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        curl \
        libportaudio2 \
        ca-certificates \
        libpulse0 \
        pulseaudio-utils \
        alsa-utils \
        libasound2-plugins \
        sox \
        libsdl2-2.0-0 \
        ffmpeg \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g @anthropic-ai/claude-code \
    && apt-get purge -y --auto-remove curl \
    && rm -rf /var/lib/apt/lists/*

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN printf 'pcm.default pulse\nctl.default pulse\n' > /etc/asound.conf

WORKDIR /app

COPY --from=builder /app/target/release/voice_assistant ./

ENV RUST_MIN_STACK=16777216

ENTRYPOINT ["./voice_assistant"]
