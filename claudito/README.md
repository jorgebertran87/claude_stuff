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
```

### Flow

```
Microphone
    │
    ▼
MicrophoneCapturer.capture()       4-second chunks, pause_threshold=2s
    │  loops until wake word heard
    ▼
GoogleTranscriber.transcribe()     Google Speech Recognition API
    │
    ▼
WakeWord.matches(text)? ──No──► loop back
    │ Yes
    ▼
MicrophoneCapturer.capture()       timeout=5s, no phrase time limit
    │  ends on 2 seconds of silence
    ▼
GoogleTranscriber.transcribe()
    │
    ▼
ClaudeCodeHandler.handle(order)    claude-haiku-4-5 via Claude Code CLI
    │  tools: Read, Write, Edit, Bash, Glob, Grep, WebSearch
    │  session persisted across orders
    ▼
GTTSSpeaker.speak(response)        markdown stripped → Google TTS → pygame
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

All configuration lives in a `.env` file in the project root.

| Variable | Default | Description |
|---|---|---|
| `VOICE_LANGUAGE` | `es-ES` | BCP-47 language code for speech recognition and TTS |
| `WAKE_WORD` | `claudito` | Word that activates order listening |
| `DEFAULT_USER_CITY` | _(auto-detected via IP)_ | Default city for weather queries with no location specified |
| `CLAUDE_SESSION_ID` | _(none)_ | Resume a specific Claude Code session across restarts |

---

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
4. Speak your order — capture ends automatically after 2 seconds of silence
5. Claude processes the order and speaks the response
6. Repeat from step 3, or press `Ctrl+C` to quit

### Example orders

```
"Claudito, qué tiempo hace en Sevilla?"
"Claudito, cuánto es 347 por 19?"
"Claudito, qué es la arquitectura hexagonal?"
"Claudito, busca en Google las noticias de hoy"
```

---

## Key implementation details

**Wake word detection** uses 4-second audio chunks with no timeout so it listens indefinitely. Only the wake word phase uses chunked listening — once triggered, the order is captured in a single continuous recording.

**Order capture** has no hard time limit (`phrase_time_limit=None`). It ends when 2 seconds of silence are detected (`pause_threshold=2.0`), allowing for natural pauses mid-sentence.

**Claude session** is preserved across orders within the same `make run` invocation. The session ID is printed on first use and can be saved as `CLAUDE_SESSION_ID` in `.env` to resume context across restarts.

**TTS preprocessing** strips markdown (links, bold, bullets, headers, code) from Claude's response before passing it to gTTS, so the spoken output is clean plain text.

**Language** is extracted from the BCP-47 code for both Google STT (`language.code` → `es-ES`) and gTTS (`language.code.split("-")[0]` → `es`).
