FROM rust:1.83-slim

# System deps: PortAudio, PulseAudio, Node.js (for Claude Code CLI)
RUN apt-get update && apt-get install -y --no-install-recommends \
        portaudio19-dev \
        ca-certificates \
        gcc \
        pkg-config \
        libpulse-dev \
        pulseaudio-utils \
        alsa-utils \
        sox \
        libasound2-dev \
        libasound2-plugins \
        libsdl2-dev \
        ffmpeg \
        curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g @anthropic-ai/claude-code \
    && rm -rf /var/lib/apt/lists/*

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN echo 'pcm.default pulse\nctl.default pulse' > /etc/asound.conf

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY tests/ tests/

ENV RUST_MIN_STACK=16777216
RUN cargo build --release --jobs $(nproc)

ENTRYPOINT ["./target/release/voice_assistant"]
