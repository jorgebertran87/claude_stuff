# changes_detector

A Rust service that watches web pages for changes and sends Telegram notifications with line-level diffs. Monitors are managed entirely through Telegram bot commands — no config file changes or restarts required.

## How it works

```
┌──────────────────┐   fetch()   ┌──────────────────┐   diff   ┌──────────────────┐
│  BrowserSource   │ ──────────► │  ChangeDetector  │ ───────► │    Telegram      │
│  headless Chrome │             │  hash · persist  │          │   notification   │
└──────────────────┘             └──────────────────┘          └──────────────────┘
```

Each monitor:
1. Navigates to its URL using a headless Chrome browser (handles JS-rendered SPAs).
2. Extracts a CSS-selected element (or checks whether it exists).
3. Hashes the result and compares it against the last saved snapshot.
4. Sends a Telegram diff if anything changed.

Monitors are persisted to `/data/monitors.json` and resume automatically on restart.

## Bot commands

| Command | Description |
|---|---|
| `/add` | Add a new monitor (guided 5-step conversation) |
| `/remove` | Stop and remove a running monitor |
| `/status` | List all configured monitors |
| `/cancel` | Abort an in-progress `/add` or `/remove` |

### `/add` walkthrough

The bot asks five questions in sequence:

| Step | What you send | Notes |
|---|---|---|
| 1 | **Alias** | Short name, e.g. `match-456` |
| 2 | **URL** | Must start with `http://` or `https://` |
| 3 | **CSS selector** | e.g. `[id="456"] .buy-btn` |
| 4 | **Mode** — `content` or `exists` | See below |
| 5 | **Interval** | Check frequency in seconds, e.g. `60` |

**Monitoring modes:**

- `content` — notifies when the element's HTML changes (text, attributes, structure).
- `exists` — notifies when the element appears on or disappears from the page (`present` ↔ `absent`).

> **CSS selector tip:** IDs that start with a digit cannot use `#N` notation.
> Use the attribute form instead: `[id="237"]` ✔ — `#237` ✘

### `/status` output

```
✅ Bot is running

📋 Monitors (2):

🏷 match-456
🌐 https://ticketing.example.com
🔍 [id="456"] .c-card__buy-btn
👁 content 📝 · ⏱ 60 s

🏷 vip-lock
🌐 https://ticketing.example.com
🔍 [id="237"] .fa-user-lock
👁 exists 👁 · ⏱ 30 s
```

### Telegram notification format

```
🔔 Change detected

📄 Source: https://ticketing.example.com  [[id="456"] .c-card__buy-btn]

Diff:
- <button class="buy-btn" disabled>Sold out</button>
+ <button class="buy-btn">Buy now</button>
```

## Project structure

```
src/
├── source/
│   ├── mod.rs        # Source trait — location() + async fetch() → String
│   └── browser.rs    # BrowserSource — headless Chrome via WebDriver
├── detector.rs       # ChangeDetector — SHA-256 hashing, similar line diff, state persistence
├── monitor.rs        # MonitorConfig / MonitorStore — JSON persistence of /add monitors
├── runner.rs         # MonitorSpawner, Notifier trait, shared run_loop
├── telegram.rs       # TelegramNotifier + CommandHandler (bot commands)
├── config.rs         # Configuration loaded from environment variables
└── main.rs           # Entry point — wires everything together
```

## Configuration

Copy `.env.example` to `.env` and set the two required variables.

| Variable | Required | Default | Description |
|---|---|---|---|
| `TELEGRAM_BOT_TOKEN` | ✅ | — | Bot token from [@BotFather](https://t.me/BotFather) |
| `TELEGRAM_CHAT_ID` | ✅ | — | Chat, group or channel ID for notifications |
| `WEBDRIVER_URL` | | `http://chrome:4444` | WebDriver server used by all browser monitors |
| `DATA_DIR` | | `/data` | Directory for state files and `monitors.json` |
| `RUST_LOG` | | `changes_detector=info` | Log level (`error` / `warn` / `info` / `debug`) |

> **Finding your Telegram chat ID:** forward any message to [@userinfobot](https://t.me/userinfobot) or [@RawDataBot](https://t.me/RawDataBot).

## Running with Docker Compose

```bash
# 1. Configure
cp .env.example .env
$EDITOR .env   # set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID

# 2. Start
docker compose up --build -d

# 3. Follow logs
docker compose logs -f

# 4. Add your first monitor in Telegram
# → /add
```

The `chrome` service (Selenium standalone Chrome) is started automatically alongside the bot. State is persisted in the `changes_state` Docker volume so all monitors survive restarts.

## Architecture notes

- **BrowserSource** opens a fresh Chrome session on every fetch — no cross-poll caching.
- **Content mode** waits up to 40 s for the element to appear, then extracts `outerHTML` with collapsed whitespace. Attribute changes (e.g. icon class swaps) are captured.
- **Existence mode** waits up to 25 s for the element to appear. Returns `"present"` or `"absent"`; any state flip triggers a notification.
- **MonitorSpawner** tracks an `AbortHandle` per monitor so `/remove` can stop any task at runtime without restarting the service.
- Each monitor stores its last snapshot in `/data/<alias>.state`, keyed by SHA-256 hash.
