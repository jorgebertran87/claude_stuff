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
│   ├── file.rs       # FileSource   — reads a local file
│   └── http.rs       # HttpSource  — fetches a URL, optional CSS selector
├── detector.rs       # ChangeDetector — hashing, diffing, state persistence
├── telegram.rs       # TelegramNotifier — Telegram Bot API (HTML mode)
├── config.rs         # Configuration loaded from environment variables
└── main.rs           # Wires everything together; polling loop
```

## Configuration

Copy `.env.example` to `.env` and fill in the required values.

The source type is inferred automatically from `MONITOR_TARGET`:

| `MONITOR_TARGET` value | Source used |
|---|---|
| Starts with `http://` or `https://` | `HttpSource` |
| Any other value | `FileSource` (path inside the container) |

### All variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `MONITOR_TARGET` | ✅ | — | URL **or** file path to monitor |
| `TELEGRAM_BOT_TOKEN` | ✅ | — | Bot token from [@BotFather](https://t.me/BotFather) |
| `TELEGRAM_CHAT_ID` | ✅ | — | Chat, group or channel ID for notifications |
| `HTML_SELECTOR` | | _(full body)_ | CSS selector to narrow monitoring to a specific element (HTTP only) |
| `CHECK_INTERVAL_SECS` | | `60` | Polling interval in seconds |
| `STATE_FILE` | | `/data/<slug>.state` | Path where the snapshot is persisted |
| `RUST_LOG` | | `changes_detector=info` | Log level |

> **Note on CSS IDs that start with a digit:** the selector `#237` is invalid CSS. Use the attribute form instead: `a[id="237"]`.

> **Finding your Telegram chat ID:** forward any message to [@userinfobot](https://t.me/userinfobot) or [@RawDataBot](https://t.me/RawDataBot).

## Running with Docker Compose

```bash
# 1. Configure
cp .env.example .env
$EDITOR .env   # set MONITOR_TARGET, TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_ID

# 2. Start
docker compose up --build -d

# 3. Follow logs
docker compose logs -f
```

State is persisted in the `changes_state` Docker volume so the service resumes correctly after restarts.

## Running locally (without Docker)

```bash
cargo build --release

# Monitor an HTTP element
MONITOR_TARGET=https://ticketing.rcdeportivo.es/ \
HTML_SELECTOR='a[id="237"]' \
TELEGRAM_BOT_TOKEN=<token> \
TELEGRAM_CHAT_ID=<chat_id> \
./target/release/changes_detector

# Monitor a local file
MONITOR_TARGET=./watched/example.txt \
TELEGRAM_BOT_TOKEN=<token> \
TELEGRAM_CHAT_ID=<chat_id> \
./target/release/changes_detector
```

## Telegram notification format

```
🔔 File change detected

📄 File: https://ticketing.rcdeportivo.es/  [a[id="237"]]

Diff:
- Buy tickets
+ Sold out
```

## Adding a new Source

1. Create `src/source/<name>.rs` and implement the `Source` trait:

```rust
use async_trait::async_trait;
use super::Source;

pub struct MySource { location: String }

#[async_trait]
impl Source for MySource {
    fn location(&self) -> &str { &self.location }

    async fn fetch(&self) -> anyhow::Result<String> {
        // return the current string to monitor
        todo!()
    }
}
```

2. Expose it in `src/source/mod.rs`:

```rust
pub mod my_source;
```

3. Add a branch in `main.rs` (the only place that knows about infrastructure):

```rust
let source: Box<dyn Source> = Box::new(MySource::new(cfg.monitor_target));
```

Nothing else changes — the detector, notifier, and polling loop are all source-agnostic.
