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
COPY deepseek_client/ deepseek_client/
COPY skill_loader/ skill_loader/
RUN mkdir -p src deepseek_client/src skill_loader/src \
    && touch src/lib.rs \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > deepseek_client/src/lib.rs \
    && echo '' > skill_loader/src/lib.rs \
    && cargo build --release \
    && rm -rf src \
              deepseek_client/src \
              skill_loader/src \
              target/release/.fingerprint/voice_assistant-* \
              target/release/deps/voice_assistant-* \
              target/release/deps/libvoice_assistant-* \
              target/release/voice_assistant

# Compile application sources
COPY src/ src/
COPY deepseek_client/src/ deepseek_client/src/
COPY skill_loader/src/ skill_loader/src/
RUN touch src/lib.rs src/main.rs deepseek_client/src/lib.rs skill_loader/src/lib.rs \
    && cargo build --release

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

# Piper TTS version (pre-built binary from GitHub releases).
ARG PIPER_VERSION=2023.11.14-2

RUN apt-get update && apt-get install -y --no-install-recommends \
        curl \
        libportaudio2 \
        ca-certificates \
        libpulse0 \
        pulseaudio-utils \
        alsa-utils \
        libasound2-plugins \
        ffmpeg \
        libgomp1 \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g @anthropic-ai/claude-code \
    && rm -rf /var/lib/apt/lists/*

# Install Piper TTS binary (offline text-to-speech).
RUN mkdir -p /app/piper_models \
    && curl -fsSL "https://github.com/rhasspy/piper/releases/download/${PIPER_VERSION}/piper_linux_x86_64.tar.gz" \
        | tar xz -C /usr/local/bin \
    && chmod +x /usr/local/bin/piper

# Download Spanish voice model.
RUN curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/carlfm/x_low/es_ES-carlfm-x_low.onnx" \
        -o /app/piper_models/es_ES-carlfm-x_low.onnx \
    && curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/carlfm/x_low/es_ES-carlfm-x_low.onnx.json" \
        -o /app/piper_models/es_ES-carlfm-x_low.onnx.json

# Download English fallback voice model.
RUN curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx" \
        -o /app/piper_models/en_US-lessac-medium.onnx \
    && curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json" \
        -o /app/piper_models/en_US-lessac-medium.onnx.json

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN printf 'pcm.default pulse\nctl.default pulse\n' > /etc/asound.conf

WORKDIR /app

COPY --from=builder /app/target/release/voice_assistant ./

ENV RUST_MIN_STACK=16777216

ENTRYPOINT ["./voice_assistant"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD pgrep voice_assistant || exit 1
