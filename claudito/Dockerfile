FROM python:3.12-slim

# System deps: PortAudio for PyAudio + CA certs for Google STT HTTPS
RUN apt-get update && apt-get install -y --no-install-recommends \
        portaudio19-dev \
        ca-certificates \
        gcc \
        python3-dev \
        libpulse0 \
        pulseaudio-utils \
        libasound2-plugins \
    && rm -rf /var/lib/apt/lists/*

# Route ALSA through PulseAudio (which PipeWire emulates)
RUN echo 'pcm.default pulse\nctl.default pulse' > /etc/asound.conf

WORKDIR /app

RUN pip install --no-cache-dir \
    SpeechRecognition \
    pyaudio

COPY voice_listener/ voice_listener/
COPY debug_audio.py .

CMD ["python", "-m", "voice_listener"]
