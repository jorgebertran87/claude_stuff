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

# ── Stage 2: node-installer (shared between test-base and runtime) ────────────
FROM debian:bookworm-slim AS node-installer

RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs npm \
    && npm install -g @anthropic-ai/claude-code \
    && apt-get purge -y --auto-remove curl \
    && rm -rf /var/lib/apt/lists/*

# ── Stage 3: test-base (builder + Node/claude + merged Cargo manifest) ────────
FROM builder AS test-base

RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs npm \
    && npm install -g @anthropic-ai/claude-code \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo-test.toml ./
RUN cat Cargo-test.toml >> Cargo.toml

# ── Stage 4: test (test-base + test files) ────────────────────────────────────
FROM test-base AS test

COPY tests/ tests/
COPY features/ features/
COPY test_speech_es.wav test_speech_en.wav ./

# ── Stage 5: mutants (test-base + cargo-mutants + .cargo/ + test files) ───────
FROM test-base AS mutants

RUN cargo install --locked cargo-mutants@26.0.0

COPY .cargo/ .cargo/
COPY tests/ tests/
COPY features/ features/
COPY test_speech_es.wav test_speech_en.wav ./

# ── Stage 6: runtime ──────────────────────────────────────────────────────────
FROM node-installer

RUN apt-get update && apt-get install -y --no-install-recommends \
        libportaudio2 \
        ca-certificates \
        libpulse0 \
        pulseaudio-utils \
        alsa-utils \
        libasound2-plugins \
        sox \
        libsdl2-2.0-0 \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN printf 'pcm.default pulse\nctl.default pulse\n' > /etc/asound.conf

WORKDIR /app

COPY --from=builder /app/target/release/voice_assistant ./

ENV RUST_MIN_STACK=16777216

ENTRYPOINT ["./voice_assistant"]
