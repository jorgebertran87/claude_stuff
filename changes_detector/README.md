# changes_detector

A Rust service that watches web pages for changes and sends Telegram notifications with line-level diffs. Monitors are managed entirely through Telegram bot commands — no config file changes or restarts required.

## How it works

```
┌──────────────────────┐   fetch()   ┌──────────────────┐   diff   ┌──────────────────┐
│  BrowserSource  or   │ ──────────► │  ChangeDetector  │ ───────► │    Telegram      │
│  FlareSolverSource   │             │  hash · persist  │          │   notification   │
└──────────────────────┘             └──────────────────┘          └──────────────────┘
```

Each monitor:
1. Navigates to its URL using the configured fetch backend (headless Chrome or FlareSolverr).
2. Extracts a CSS-selected element's content, or checks whether it exists.
3. Hashes the result and compares it against the last saved snapshot.
4. Sends a Telegram diff with a link to the page if anything changed.

Monitors are persisted to `/data/monitors.json` and resume automatically on restart.

## Bot commands

| Command | Description |
|---|---|
| `/add` | Add a new monitor (guided 6-step conversation) |
| `/remove` | Stop and permanently remove a monitor |
| `/pause <alias>` | Suspend polling without deleting the monitor |
| `/resume <alias>` | Restart a paused monitor |
| `/status` | List all monitors with their state |
| `/check <alias>` | Fetch the current text content of a monitor's selector |
| `/cancel` | Abort an in-progress `/add` or `/remove` conversation |

---

### `/add` walkthrough

The bot asks six questions in sequence:

| Step | What you send | Notes |
|---|---|---|
| 1 | **Alias** | Short name, e.g. `match-456` |
| 2 | **URL** | Must start with `http://` or `https://` |
| 3 | **Fetch method** — `browser` or `flare` | See below |
| 4 | **CSS selector** | e.g. `[data-t="threadLink"]` |
| 5 | **Mode** — `content` or `exists` | See below |
| 6 | **Interval** | Check frequency in seconds, e.g. `60` |

**Fetch methods:**

- `browser` — headless Chrome via Selenium. Works for most sites.
- `flare` — FlareSolverr. Use this for sites protected by Cloudflare JS challenges (403 errors or challenge pages in browser mode).

**Monitoring modes:**

- `content` — notifies when the element's HTML changes (text, attributes, structure).
- `exists` — notifies when the element appears on or disappears from the page (`present` ↔ `absent`).

> **CSS selector tip:** IDs that start with a digit cannot use `#N` notation.
> Use the attribute form instead: `[id="237"]` ✔ — `#237` ✘

---

### `/status` output

```
✅ Bot is running

📋 Monitors (2):

▶️ match-456
🌐 https://ticketing.example.com
⚙️ browser 🌐 · 👁 content 📝 · ⏱ 60 s
🔍 [id="456"] .c-card__buy-btn

⏸ deals-justeat
🌐 https://www.chollometro.com/search?q=just+eat
⚙️ flare 🔥 · 👁 content 📝 · ⏱ 300 s
🔍 [data-t="threadLink"]
```

`▶️` = running · `⏸` = paused

---

### `/check <alias>`

Fetches the current text content of the selector immediately, without waiting for the next polling cycle.

```
🔍 deals-justeat
🔗 https://www.chollometro.com/search?q=just+eat

Just Eat — 50% descuento en tu primer pedido · Ahorra hasta 15€…
```

---

### `/pause` and `/resume`

```
/pause deals-justeat   → ⏸ Monitor paused (config kept, polling stopped)
/resume deals-justeat  → ▶️ Monitor resumed
```

Paused state is persisted — the monitor stays paused across container restarts.

---

### Telegram notification format

```
🔔 Change detected

📄 Source: https://www.chollometro.com/search?q=just+eat  [[data-t="threadLink"]]
🔗 Open page

Diff:
- Just Eat — 40% descuento…
+ Just Eat — 50% descuento…
```

---

## Project structure

```
src/
├── source/
│   ├── mod.rs        # Source trait — location() + async fetch() → String
│   ├── browser.rs    # BrowserSource — headless Chrome via WebDriver
│   └── flare.rs      # FlareSolverSource — Cloudflare bypass via FlareSolverr API
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
| `WEBDRIVER_URL` | | `http://chrome:4444` | WebDriver server used by `browser`-mode monitors |
| `FLARESOLVERR_URL` | | `http://flaresolverr:8191` | FlareSolverr API used by `flare`-mode monitors |
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

Three services start automatically:
- **`changes_detector`** — the bot itself.
- **`chrome`** — Selenium standalone Chrome for `browser`-mode monitors.
- **`flaresolverr`** — Cloudflare bypass proxy for `flare`-mode monitors.

State is persisted in the `changes_state` Docker volume so all monitors survive restarts.

## Architecture notes

- **BrowserSource** opens a fresh Chrome session on every fetch — no cross-poll caching. Sets `--disable-blink-features=AutomationControlled` and masks `navigator.webdriver` to reduce bot-detection fingerprinting.
- **FlareSolverSource** POSTs to the FlareSolverr `/v1` API, which uses an undetected Chrome to solve Cloudflare challenges, then parses the returned HTML with `scraper`. No live browser session is maintained between polls.
- **Content mode** waits up to 40 s (browser) or 90 s (flare) for the element, then returns `outerHTML` with collapsed whitespace. Attribute changes (e.g. icon class swaps) are captured.
- **Existence mode** polls `querySelector` for up to 25 s then returns `"present"` or `"absent"`; any state flip triggers a notification.
- **MonitorSpawner** tracks an `AbortHandle` per monitor — `/pause` and `/remove` abort tasks at runtime without restarting the service. `/resume` re-spawns from the persisted config.
- Each monitor stores its last snapshot in `/data/<alias>.state`, keyed by SHA-256 hash.
