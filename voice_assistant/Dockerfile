# ── Stage 1: builder ──────────────────────────────────────────────────────────
FROM rust:1.86-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        portaudio19-dev \
        gcc \
        pkg-config \
        libpulse-dev \
        libasound2-dev \
        libsdl2-dev \
        libssl-dev \
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

# ── Stage 2: tools (Piper TTS binary and voice models) ────────────────────
FROM debian:bookworm-slim AS tools

ARG PIPER_VERSION=2023.11.14-2

RUN apt-get update && apt-get install -y --no-install-recommends \
        curl \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Piper TTS binary.
RUN mkdir -p /app/piper_models \
    && curl -fsSL "https://github.com/rhasspy/piper/releases/download/${PIPER_VERSION}/piper_linux_x86_64.tar.gz" \
        | tar xz -C /usr/local/bin \
    && chmod +x /usr/local/bin/piper

# Spanish voice model (sharvard-medium — best quality Spanish voice).
RUN curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/sharvard/medium/es_ES-sharvard-medium.onnx" \
        -o /app/piper_models/es_ES-sharvard-medium.onnx \
    && curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/sharvard/medium/es_ES-sharvard-medium.onnx.json" \
        -o /app/piper_models/es_ES-sharvard-medium.onnx.json

# English fallback voice model.
RUN curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx" \
        -o /app/piper_models/en_US-lessac-medium.onnx \
    && curl -fsSL "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json" \
        -o /app/piper_models/en_US-lessac-medium.onnx.json

# ── Stage 3: whisper-builder (compile whisper.cpp from source) ────────────
FROM debian:bookworm-slim AS whisper-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        git \
        build-essential \
        cmake \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN git clone --depth 1 --branch v1.7.4 https://github.com/ggerganov/whisper.cpp.git /tmp/whisper.cpp \
    && cd /tmp/whisper.cpp \
    && cmake -B build -DCMAKE_BUILD_TYPE=Release \
    && cmake --build build --config Release -j "$(nproc)" \
    && cp build/bin/whisper-cli /usr/local/bin/whisper-cli \
    && find build -name '*.so*' -exec cp {} /usr/local/lib/ \; \
    && rm -rf /tmp/whisper.cpp

# Download ggml-tiny model.
RUN mkdir -p /app/models \
    && apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && curl -fsSL "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin" \
        -o /app/models/ggml-tiny.bin \
    && apt-get purge -y curl && apt-get autoremove -y && rm -rf /var/lib/apt/lists/*

# ── Stage 4: test-base (builder + Piper tools + whisper-cli + merged Cargo manifest)
FROM builder AS test-base

# Piper needs libgomp1 at runtime (not in rust:1.86-slim by default).
RUN apt-get update && apt-get install -y --no-install-recommends \
        libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Copy Piper from the tools stage so integration tests can use it.
COPY --from=tools /usr/local/bin/piper          /usr/local/bin/
COPY --from=tools /app/piper_models             /app/piper_models/

# whisper-cli for WhisperTranscriber integration tests
COPY --from=whisper-builder /usr/local/bin/whisper-cli /usr/local/bin/
COPY --from=whisper-builder /usr/local/lib/            /usr/local/lib/
COPY --from=whisper-builder /app/models                /app/models/
RUN ldconfig

COPY config/Cargo-test.toml ./
RUN cat Cargo-test.toml >> Cargo.toml

# ── Stage 5: test (test-base + test files) ────────────────────────────────────
FROM test-base AS test

COPY tests/unit/ tests/unit/
COPY tests/integration/ tests/integration/
COPY features/ features/
COPY tests/files/ tests/files/
COPY .claude/commands/ .claude/commands/

# ── Stage 6: mutants (test-base + cargo-mutants + test files) ─────────────────
FROM test-base AS mutants

RUN cargo install --locked cargo-mutants@26.0.0

COPY config/mutants.toml config/mutants-integration.toml config/
COPY tests/unit/ tests/unit/
COPY tests/integration/ tests/integration/
COPY features/ features/
COPY tests/files/ tests/files/

# ── Stage 7: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

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
        procps \
    && rm -rf /var/lib/apt/lists/*

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN printf 'pcm.default pulse\nctl.default pulse\n' > /etc/asound.conf

WORKDIR /app

COPY --from=tools             /usr/local/bin/piper          /usr/local/bin/
COPY --from=tools             /app/piper_models             /app/piper_models/
COPY --from=whisper-builder   /usr/local/bin/whisper-cli    /usr/local/bin/
COPY --from=whisper-builder   /usr/local/lib/               /usr/local/lib/
COPY --from=whisper-builder   /app/models                   /app/models/
COPY --from=builder           /app/target/release/voice_assistant ./

RUN ldconfig

ENV RUST_MIN_STACK=16777216

ENTRYPOINT ["./voice_assistant"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD pgrep voice_assistant || exit 1
