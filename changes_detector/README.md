# changes_detector

A lightweight Rust service that polls a source for text changes and sends a Telegram notification with a line-level diff whenever the content changes.

## How it works

```
┌─────────────┐   fetch()   ┌──────────────────┐   diff   ┌──────────────────┐
│   Source    │ ──────────► │  ChangeDetector  │ ───────► │    Telegram      │
│  (any impl) │             │  hash · persist  │          │   notification   │
└─────────────┘             └──────────────────┘          └──────────────────┘
```

1. Every `CHECK_INTERVAL_SECS` seconds the configured **Source** is fetched.
2. The **ChangeDetector** hashes the result and compares it against the last persisted snapshot.
3. If the content changed, a `+`/`-` line diff is sent to a Telegram chat.

The source and the detection logic are fully decoupled — adding a new kind of source (HTTP endpoint, S3 object, database query…) only requires a new file under `src/source/`.

## Project structure

```
src/
├── source/
│   ├── mod.rs        # Source trait  — location() + async fetch() → String
│   └── file.rs       # FileSource   — reads a local file
├── detector.rs       # ChangeDetector — hashing, diffing, state persistence
├── telegram.rs       # TelegramNotifier — Telegram Bot API (HTML mode)
├── config.rs         # Configuration loaded from environment variables
└── main.rs           # Wires everything together; polling loop
```

## Configuration

Copy `.env.example` to `.env` and fill in the required values:

| Variable | Required | Default | Description |
|---|---|---|---|
| `MONITOR_TARGET` | ✅ | — | Path to the file to monitor (inside the container) |
| `TELEGRAM_BOT_TOKEN` | ✅ | — | Bot token from [@BotFather](https://t.me/BotFather) |
| `TELEGRAM_CHAT_ID` | ✅ | — | Chat, group or channel ID to send notifications to |
| `CHECK_INTERVAL_SECS` | | `60` | Polling interval in seconds |
| `STATE_FILE` | | `/data/<slug>.state` | Path where the snapshot is persisted |
| `RUST_LOG` | | `changes_detector=info` | Log level |

> **Finding your chat ID:** forward any message to [@userinfobot](https://t.me/userinfobot) or [@RawDataBot](https://t.me/RawDataBot).

## Running with Docker Compose

```bash
# 1. Configure
cp .env.example .env
$EDITOR .env

# 2. Point to the file you want to watch (default: ./watched/example.txt)
export MONITOR_FILE_HOST=/absolute/path/to/your/file

# 3. Start
docker compose up --build -d

# 4. Follow logs
docker compose logs -f
```

State is persisted in the `changes_state` Docker volume so the service resumes correctly after restarts.

## Running locally (without Docker)

```bash
cargo build --release
MONITOR_TARGET=./watched/example.txt \
TELEGRAM_BOT_TOKEN=<token> \
TELEGRAM_CHAT_ID=<chat_id> \
./target/release/changes_detector
```

## Telegram notification format

```
🔔 File change detected

📄 File: /watched/target.txt

Diff:
- old line
+ new line
```

## Adding a new Source

1. Create `src/source/<name>.rs` and implement the `Source` trait:

```rust
use async_trait::async_trait;
use crate::source::Source;

pub struct HttpSource { url: String }

#[async_trait]
impl Source for HttpSource {
    fn location(&self) -> &str { &self.url }

    async fn fetch(&self) -> anyhow::Result<String> {
        Ok(reqwest::get(&self.url).await?.text().await?)
    }
}
```

2. Expose it in `src/source/mod.rs`:

```rust
pub mod http;
```

3. Wire it in `main.rs` (the only place that knows about infrastructure):

```rust
let source: Box<dyn Source> = Box::new(HttpSource::new(cfg.monitor_target));
```

Nothing else changes — the detector, notifier, and polling loop are all source-agnostic.
