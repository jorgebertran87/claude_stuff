FROM python:3.12-slim

# System deps: PortAudio, PulseAudio, Node.js (for Claude Code CLI)
RUN apt-get update && apt-get install -y --no-install-recommends \
        portaudio19-dev \
        ca-certificates \
        gcc \
        python3-dev \
        libpulse0 \
        pulseaudio-utils \
        libasound2-plugins \
        libsdl2-2.0-0 \
        ffmpeg \
        curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g @anthropic-ai/claude-code \
    && rm -rf /var/lib/apt/lists/*

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN echo 'pcm.default pulse\nctl.default pulse' > /etc/asound.conf

WORKDIR /app

RUN pip install --no-cache-dir \
    SpeechRecognition \
    pyaudio \
    claude-agent-sdk \
    gTTS \
    pygame \
    numpy \
    noisereduce \
    pydub

COPY voice_listener/ voice_listener/
COPY debug_audio.py .
COPY prompt* .

CMD ["python", "-m", "voice_listener"]
