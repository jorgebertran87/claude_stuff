# Claudito — Voice Assistant powered by Claude Code

Claudito listens for a wake word, captures a voice order, sends it to Claude Code CLI for processing, and speaks the response aloud. It runs fully inside Docker with microphone input and audio output routed through the host's PipeWire/PulseAudio server.

---

## Architecture

The project follows **Domain-Driven Design (DDD)**. The domain layer never imports from infrastructure.

```
voice_listener/
├── domain/
│   ├── model.py          # Value objects: WakeWord, Language, AudioCapture
│   ├── ports.py          # Abstract ports: AudioCapturer, Transcriber, OrderHandler, AudioSpeaker
│   └── service.py        # VoiceListenerService — orchestration logic
└── infrastructure/
    ├── audio.py          # MicrophoneCapturer  → AudioCapturer
    ├── speech.py         # GoogleTranscriber   → Transcriber
    ├── claude_handler.py # ClaudeCodeHandler   → OrderHandler
    └── speaker.py        # GTTSSpeaker         → AudioSpeaker

system_prompt.txt         # Claude system prompt template (uses {default_user_city}, {voice_language})
```

### Flow

```
Microphone
    │
    ▼
MicrophoneCapturer.capture()       8-second chunks, indefinite timeout
    │  loops until wake word heard
    ▼
GoogleTranscriber.transcribe()     Google Speech Recognition API
    │
    ▼
WakeWord.matches(text)? ──No──► loop back
    │ Yes (fuzzy match, threshold 0.80)
    ▼
MicrophoneCapturer.capture()       timeout=10s, ends after 4s of silence
    │  up to 2 retries if nothing heard
    ▼
GoogleTranscriber.transcribe()
    │
    ▼
ClaudeCodeHandler.handle(order)    claude-haiku-4-5 via Claude Agent SDK
    │  tools: Read, Write, Edit, Bash, Glob, Grep, WebSearch
    │  session persisted across orders
    ▼
MicrophoneCapturer.mute()          mic stream paused during playback
    │
    ▼
GTTSSpeaker.speak(response)        markdown stripped → Google TTS → pydub (1.5x speed, 0.75x pitch) → pygame
    │
    ▼
MicrophoneCapturer.unmute()
    │
    ▼
loop back to wake word detection
```

---

## Requirements

- Docker
- Host running PipeWire or PulseAudio
- Claude Code CLI authenticated on the host (`claude login`)

---

## Configuration

All configuration lives in a `.env` file in the project root (see `.env.example`).

| Variable | Default | Description |
|---|---|---|
| `VOICE_LANGUAGE` | `es-ES` | BCP-47 language code for speech recognition and TTS |
| `WAKE_WORD` | `claudito` | Word that activates order listening |
| `DEFAULT_USER_CITY` | _(required)_ | Default city for weather queries with no location specified |
| `CLAUDE_SESSION_ID` | _(none)_ | Resume a specific Claude Code session across restarts |

### `.env.example`

```env
DEFAULT_USER_CITY=xxx
VOICE_LANGUAGE=es-ES
WAKE_WORD=Claudito
```

## Running with Docker

### Build the image

```bash
make build
```

### Run

```bash
make run
```

The container mounts:
| Mount | Purpose |
|---|---|
| `/run/user/$UID/pulse/native` | PipeWire/PulseAudio socket — microphone input and audio output |
| `~/.claude/` | Claude Code session history and config |
| `~/.claude.json` | Claude Code authentication |
| `.env` | Application environment variables |

### Debug audio devices

```bash
make debug
```

Lists all audio input/output devices detected inside the container. Useful if the microphone is not being picked up.

---

## Usage

1. Run `make run`
2. Wait for: `Waiting for wake word "claudito"...`
3. Say **"Claudito"** — the app prints `Wake word detected!`
4. Speak your order — capture ends automatically after 4 seconds of silence
5. Claude processes the order and speaks the response
6. Repeat from step 3, or press `Ctrl+C` to quit

### Inline orders

The wake word and the order can be said in one breath:

> "Claudito, qué tiempo hace en Sevilla?"

If the wake word is recognised alone, a beep prompts you to speak the order separately.

### Example orders

```
"Claudito, qué tiempo hace en Sevilla?"
"Claudito, cuánto es 347 por 19?"
"Claudito, qué es la arquitectura hexagonal?"
"Claudito, busca en Google las noticias de hoy"
```

---

## Key implementation details

**Wake word detection** uses 8-second audio chunks with no timeout so it listens indefinitely. Matching is fuzzy (`SequenceMatcher` ratio ≥ 0.80) to handle slight mispronunciations. If the utterance contains words after the wake word, they are extracted as an inline order and the separate order-capture step is skipped.

**Order capture** has no hard time limit (`phrase_time_limit=None`). It ends when 4 seconds of silence are detected (`pause_threshold=4.0`), allowing for natural pauses mid-sentence. Up to 2 retries are attempted before giving up.

**Microphone muting** — the mic audio stream is paused via PyAudio (`stop_stream`) while the speaker is playing and resumed immediately after (`start_stream`). This prevents speaker bleed from accumulating in the mic buffer and triggering false wake-word detections.

**Audio processing** — the gTTS MP3 response is processed with `pydub` before playback: first sped up 1.5× (pitch-preserving resample), then pitch-shifted down to 0.75× (deeper voice). The result is exported as WAV and played via pygame. `ffmpeg` is required as the pydub MP3 backend.

**Claude session** is preserved across orders within the same `make run` invocation. The session ID is printed on first use and can be saved as `CLAUDE_SESSION_ID` in `.env` to resume context across restarts.

**TTS preprocessing** strips markdown (links, bold, bullets, headers, inline code) from Claude's response before passing it to gTTS, so the spoken output is clean plain text.

**Ambient noise calibration** runs for 1 second at startup and fixes the energy threshold. If the detected threshold exceeds 17000 (noisy environment), calibration retries automatically.

**Language** is extracted from the BCP-47 code for both Google STT (`language.code` → `es-ES`) and gTTS (`language.code.split("-")[0]` → `es`).
