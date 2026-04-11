# ── Stage 1: build ────────────────────────────────────────────────────────────
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

COPY Cargo.toml Cargo.lock* build.rs ./
COPY src/ src/

ENV RUST_MIN_STACK=16777216
RUN cargo build --release

# ── Stage 1b: test (builder + claude CLI) ─────────────────────────────────────
FROM builder AS test

RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g @anthropic-ai/claude-code \
    && rm -rf /var/lib/apt/lists/*

# Frequently changing files last
COPY Cargo-test.toml Cargo.toml ./
RUN echo "" >> Cargo.toml
RUN cat Cargo-test.toml >> Cargo.toml
COPY .cargo/ .cargo/
COPY tests/ tests/
COPY features/ features/
COPY test_speech_es.wav test_speech_en.wav ./

# ── Stage 1c: mutants (test + cargo-mutants) ───────────────────────────────────
FROM test AS mutants

RUN cargo install --locked cargo-mutants@26.0.0

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

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
        curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g @anthropic-ai/claude-code \
    && rm -rf /var/lib/apt/lists/*

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN printf 'pcm.default pulse\nctl.default pulse\n' > /etc/asound.conf

WORKDIR /app

COPY --from=builder /app/target/release/voice_assistant ./

ENV RUST_MIN_STACK=16777216

ENTRYPOINT ["./voice_assistant"]
