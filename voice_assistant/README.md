# Claudito — Voice Assistant powered by Claude Code

Claudito listens for a wake word, captures a voice order, sends it to Claude Code CLI for processing, and speaks the response aloud. It runs fully inside Docker with microphone input and audio output routed through the host's PipeWire/PulseAudio server.

---

## Repository layout

```
voice_assistant/
├── features/                         # BDD feature specs
│   ├── audio_processing.feature
│   ├── claude_handler_token_logging.feature
│   ├── conversation_flow.feature
│   ├── interruptible_speech.feature
│   ├── order_capture.feature
│   ├── tts_pipeline.feature
│   └── wake_word_detection.feature
└── rust/                             # Rust implementation
    ├── src/
    │   ├── domain/
    │   │   ├── model.rs              # Value objects: WakeWord, Language, AudioCapture
    │   │   ├── ports.rs              # Traits: AudioCapturer, Transcriber, OrderHandler, AudioSpeaker
    │   │   └── service.rs            # VoiceListenerService — orchestration logic
    │   ├── cli.rs                    # CLI argument parsing (parse_args, CliArgs)
    │   └── infrastructure/
    │       ├── audio.rs              # MicrophoneCapturer  → AudioCapturer
    │       ├── speech.rs             # WhisperTranscriber  → Transcriber
    │       ├── transcriber.rs        # Transcription helpers
    │       ├── claude_handler.rs     # ClaudeCodeHandler   → OrderHandler
    │       ├── speaker.rs            # GTTSSpeaker         → AudioSpeaker
    │       └── telegram_bot.rs       # TelegramBot — long-polling text interface
    ├── tests/
    │   ├── service_tests.rs
    │   ├── claude_handler_tests.rs
    │   ├── cli_tests.rs
    │   └── direct_order_integration_tests.rs
    ├── Cargo.toml
    ├── Dockerfile
    ├── Makefile
    ├── run.sh
    └── .env.example
```

## Architecture

The implementation follows **Domain-Driven Design (DDD)**. The domain layer never imports from infrastructure.

### Flow

```
Microphone
    │
    ▼
MicrophoneCapturer.capture()       8-second chunks, indefinite timeout
    │  loops until wake word heard  (skipped if previous response was a question)
    ▼
WhisperTranscriber.transcribe()    local Whisper model
    │
    ▼
WakeWord.matches(text)? ──No──► loop back
    │ Yes (fuzzy match, threshold 0.80)
    ▼
MicrophoneCapturer.capture()       timeout=10s, ends after 2s of silence
    │  up to 2 retries if nothing heard
    ▼
WhisperTranscriber.transcribe()
    │
    ▼
GTTSSpeaker.play_melody()          waiting melody starts in background thread
    │
    ▼
ClaudeCodeHandler.handle(order)    claude-haiku-4-5 via Claude Code CLI
    │  tools: Bash, WebSearch
    │  session persisted across orders within a run
    ▼
print response to log
    │  melody still playing
    ▼
GTTSSpeaker.speak() — TTS generation (gTTS) + playback at 1.2× speed (ffplay atempo)
    │  melody still playing
    ▼
on_playback_start() fires ─────────► melody thread stopped
    │
    ▼
ffplay plays audio              mic stays active, listening for wake word
    │
    ├── wake word heard? ──Yes──► GTTSSpeaker.stop(), listen for new order
    │                              (does not wait for speech to finish)
    │
    ▼
response ended with "?"?
    ├── Yes ──► skip wake word, go straight to order capture
    └── No  ──► loop back to wake word detection
```

---

## Requirements

- Docker
- Host running PipeWire or PulseAudio
- Claude Code CLI authenticated on the host (`claude login`)

---

## Configuration

All configuration lives in `rust/.env`. See `rust/.env.example`.

| Variable | Default | Description |
|---|---|---|
| `VOICE_LANGUAGE` | `es-ES` | BCP-47 language code for speech recognition and TTS |
| `WAKE_WORD` | `claudito` | Word that activates order listening |
| `DEFAULT_USER_CITY` | _(required)_ | Default city for weather queries with no location specified |
| `TELEGRAM_BOT_TOKEN` | _(required for `--telegram`)_ | Bot token from BotFather |
| `TELEGRAM_ALLOWED_CHAT_IDS` | _(empty = allow all)_ | Comma-separated list of chat IDs that may use the bot |
| `BT_SPEAKER_MAC` | _(optional)_ | Bluetooth MAC address of the speaker; disconnected automatically after 5 min of voice inactivity |
| `CUENTAS_SHEET_NAME` | `Cuentas Personales` | Spreadsheet name shown in the analysis prompt; defaults to `Cuentas Personales` |
| `GOOGLE_SPREADSHEET_ID` | _(required for `/cuentas`)_ | ID from the Google Sheets URL (`/spreadsheets/d/<ID>/`) |
| `GOOGLE_CLIENT_ID` | _(required for `/cuentas`)_ | OAuth2 client ID from Google Cloud Console |
| `GOOGLE_CLIENT_SECRET` | _(required for `/cuentas`)_ | OAuth2 client secret |
| `GOOGLE_REFRESH_TOKEN` | _(required for `/cuentas`)_ | Long-lived refresh token (see setup below) |

### `.env.example`

```env
DEFAULT_USER_CITY=xxx
VOICE_LANGUAGE=es-ES
WAKE_WORD=Claudito
TELEGRAM_BOT_TOKEN=
TELEGRAM_ALLOWED_CHAT_IDS=
DOCKER_USERNAME=
BT_SPEAKER_MAC=
CUENTAS_SHEET_NAME=
```

## Running with Docker

Run all commands from the `rust/` folder.

### Build the image

Build a local image for the current machine (amd64):

```bash
make build
```

To build a multi-architecture image (amd64 + arm64) and push it to Docker Hub (requires `DOCKER_USERNAME` in `.env`):

```bash
make build-prod
```

### Run

```bash
make run
```

`make run` delegates to `run.sh`, which starts the container detached, streams its logs to the terminal, and stops and removes the container on exit (Ctrl+C, SIGTERM, or normal exit).

### Send a direct order (skip voice)

Pass `--order` to bypass wake word detection and transcription entirely. The order is sent straight to Claude and the response is printed to stdout:

```bash
make run ORDER="qué tiempo hace en Madrid?"
# or directly:
./run.sh --order "qué tiempo hace en Madrid?"
```

Output:

```
Order: "qué tiempo hace en Madrid?"
Claudito: <response from Claude>
```

This is useful for scripting, debugging, or quickly testing a prompt without using the microphone.

The container mounts:
| Mount | Purpose |
|---|---|
| `/run/user/$UID/pulse/native` | PipeWire/PulseAudio socket — microphone input and audio output |
| `~/.claude/` | Claude Code session history and config |
| `~/.claude.json` | Claude Code authentication |
| `.env` | Application environment variables |
| `.orders_tokens` | Append-only token/cost log |

### Telegram mode

Run the assistant as a Telegram bot instead of using the microphone:

```bash
make run-telegram
# or directly:
./run.sh --telegram
```

Set `TELEGRAM_BOT_TOKEN` in `.env` before starting. Each Telegram chat gets its own independent Claude session. Available commands:

| Command | Description |
|---|---|
| `/reset` | Clear the conversation session for the current chat |
| `/usage` | Show a summary of token usage and cost logged in `.orders_tokens` |
| `/voice_mode` | Toggle spoken audio responses for the current chat (plays through the local speaker) |
| `/volume [+N\|-N\|N]` | Adjust or query the speaker volume (e.g. `/volume 70`, `/volume +10`, `/volume`) |
| `/auth_google` | Start the Google OAuth2 flow; sends an authorization URL, then accepts the code to save the refresh token |
| `/cuentas` | Fetch the configured Google Sheet and return a Claude analysis |

If `TELEGRAM_ALLOWED_CHAT_IDS` is empty, the bot responds to any chat. Populate it with a comma-separated list of numeric chat IDs to restrict access. Messages containing only `/commands` other than the ones listed above are silently ignored. Responses longer than 4 096 characters are split and sent as multiple messages.

### Run with the published Docker Hub image

To pull the latest image from Docker Hub before running (useful on the Raspberry Pi or any machine without a local build):

```bash
make run-prod
# or in Telegram mode:
make run-telegram-prod
```

`run-prod` sets `RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE)`, which causes `run.sh` to pull the image before starting the container.

### Deploy to a remote host (Raspberry Pi)

Copy the `Makefile` and `run.sh` to a remote host configured as `pequenin` in `~/.ssh/config`:

```bash
make deploy
```

After deploying, SSH into the Pi and use `make run-prod` or `make run-telegram-prod` to pull and run the published image.

### Debug audio devices

```bash
make debug
```

Opens an interactive shell inside the container. Useful for inspecting audio devices or diagnosing microphone issues.

---

## Usage

1. Run `make run`
2. Wait for: `Waiting for wake word "claudito"...`
3. Say **"Claudito"** — the app prints `Wake word detected!`
4. Speak your order — capture ends automatically after 2 seconds of silence
5. A melody plays while Claude processes the order and prepares the audio
6. Claude speaks the response at 1.2× speed
7. If the response is a question, speak your answer directly — no wake word needed
8. While Claude is speaking, say the wake word to interrupt and ask something new
9. Repeat from step 3, or press `Ctrl+C` to quit

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

**Wake word interruption** — during playback the microphone stays active. `_speak_interruptible` runs `speak()` in a background thread while the main thread calls `capturer.capture()` in a loop (1-second timeout, 2-second phrase limit). If the transcribed audio matches the wake word, `speaker.stop()` is called immediately (kills the `ffplay` process), the speak thread is joined, and the service proceeds directly to order capture without returning to wake word detection.

**Conversation continuation** — after `speak()` finishes, if the response contains a `?` the `waiting_for_answer` flag is set. On the next iteration `wait_for_wake_word()` is skipped and `listen_for_order()` is called directly, so the user can answer naturally. If no order is captured (timeout), the flag resets and wake word detection resumes.

**Session resumption** — `ClaudeCodeHandler` tracks the Claude Code session ID returned from each `--output-format json` response. Subsequent orders within the same `make run` are sent with `--resume <session_id>`, preserving conversation context. `reset_session()` clears the stored ID; the service calls it after each complete interaction so that the next wake-word cycle starts a fresh context. In Telegram mode, `/reset` calls `reset_session()` for the relevant chat.

**Telegram bot** — `TelegramBot` runs a long-polling loop (`getUpdates` with a 30 s server timeout, 40 s HTTP timeout). Each chat ID gets its own `ClaudeCodeHandler` instance, giving every user an isolated Claude session. Unauthorised chats (when `TELEGRAM_ALLOWED_CHAT_IDS` is set) are silently dropped. Both `message` and `edited_message` update types are handled. Responses exceeding Telegram's 4 096-character limit are split on UTF-8 character boundaries and sent as sequential messages. The `TelegramGateway` trait is separated from `TelegramBot` so that tests can inject a fake gateway without any network calls.

**Waiting melody** — `_handle_with_melody` starts a `play_melody` thread before calling `handle()`. The melody is a repeating sine tone (520 Hz, 400 ms, 200 ms gap) played via `ffplay`. A shared `AtomicBool` stop signal is set inside the `on_playback_start` callback, which fires just before `ffplay` begins TTS playback, stopping the melody at the exact moment audio output starts.

**Audio processing** — TTS MP3 bytes from gTTS are written to a temp file and played via `ffplay -af atempo=1.2`, speeding up playback 1.2× without pitch distortion. No additional post-processing is applied.

**TTS chunking** — long responses are split at sentence boundaries (`.`, `!`, `?`, newlines) into chunks of ≤ 180 characters before being sent to gTTS, mirroring gTTS's own internal limit. The resulting MP3 segments are concatenated into a single byte buffer and played in one `ffplay` call.

**Alexa/Spotify handling** — if the response contains "alexa", "spotify", and a quoted song or playlist title, `alexa_spotify_title` extracts the title and detects its language. The entire voice command is then rebuilt and synthesised as a single TTS call in that language: Spanish titles produce `"Alexa, pon X en Spotify"`, English titles produce `"Alexa, play X on Spotify"`. This avoids any multilingual segment splitting and the audio gaps it caused.

**Voice mode (Telegram)** — `/voice_mode` toggles spoken audio output for a chat. When active, Claude's text responses are synthesised with gTTS and played through the local speaker (PulseAudio). Alexa+Spotify orders are always spoken regardless of voice mode. `/volume [+N|-N|N]` adjusts the PulseAudio default sink volume via `pactl` and replies with the resulting percentage.

**`/cuentas` — Google Sheets analysis** — fetches all rows from the spreadsheet identified by `GOOGLE_SPREADSHEET_ID` using the Google Sheets API v4 with an OAuth2 refresh-token flow, then forwards the data to Claude for analysis. One-time OAuth2 setup:

```bash
# 1. Create an OAuth2 credential (Desktop app) in Google Cloud Console
#    and enable the Google Sheets API.
# 2. Get the auth code URL (replace CLIENT_ID):
open "https://accounts.google.com/o/oauth2/auth?client_id=CLIENT_ID&redirect_uri=urn:ietf:wg:oauth:2.0:oob&response_type=code&scope=https://www.googleapis.com/auth/spreadsheets.readonly"
# 3. Exchange the code for tokens (replace CLIENT_ID, CLIENT_SECRET, CODE):
curl -X POST https://oauth2.googleapis.com/token \
  -d "client_id=CLIENT_ID&client_secret=CLIENT_SECRET&code=CODE&grant_type=authorization_code&redirect_uri=urn:ietf:wg:oauth:2.0:oob"
# 4. Copy the refresh_token value into .env as GOOGLE_REFRESH_TOKEN.
```

**Bluetooth auto-disconnect** — when `BT_SPEAKER_MAC` is set, a background thread checks every 30 seconds whether 5 minutes have elapsed since the last audio playback (voice mode response or Alexa+Spotify order in Telegram mode; any voice order in listen mode). If so, `bluetoothctl disconnect <MAC>` is called automatically. The timer resets after each playback and after each disconnect.

**TTS preprocessing** strips markdown (links, bold, bullets, headers, inline code) from Claude's response before passing it to gTTS, so the spoken output is clean plain text.

**Language** is extracted from the BCP-47 code for both Whisper STT (`language.code` → `es-ES`) and gTTS (`language.lang_prefix()` → `es`).

**Container lifecycle** — `run.sh` starts the container detached (`docker run -d`), traps `INT`, `TERM`, and `EXIT` to run `docker stop` + `docker rm`, and streams logs with `docker logs -f`. This ensures the container is always cleaned up on Ctrl+C or unexpected exit.
