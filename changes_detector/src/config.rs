use std::{path::PathBuf, time::Duration};

pub struct Config {
    /// Passed to the concrete `Source` implementation chosen at startup.
    pub monitor_target: String,
    /// Explicit source type override. Values: `"file"`, `"http"`, `"browser"`.
    /// When absent, the type is inferred from `monitor_target`:
    ///   http(s):// prefix → `http`
    ///   anything else     → `file`
    pub source_type: Option<String>,
    /// Optional CSS selector used by `HttpSource` and `BrowserSource`.
    /// Example: `a[id="237"]`
    pub html_selector: Option<String>,
    /// WebDriver server URL used by `BrowserSource`.
    /// Default: `http://chrome:4444` (the service name in docker-compose).
    pub webdriver_url: String,
    /// Telegram Bot token (from BotFather).
    pub telegram_bot_token: String,
    /// Telegram chat/channel ID.
    pub telegram_chat_id: String,
    /// How often to poll the source.
    pub check_interval: Duration,
    /// Where the detector persists its last-known snapshot.
    pub state_file: PathBuf,
    /// Directory used for all state files and the monitor store.
    pub data_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let monitor_target = std::env::var("MONITOR_TARGET")
            .map_err(|_| anyhow::anyhow!("MONITOR_TARGET env var is required"))?;

        let source_type = std::env::var("SOURCE_TYPE").ok();
        let html_selector = std::env::var("HTML_SELECTOR").ok();
        let webdriver_url = std::env::var("WEBDRIVER_URL")
            .unwrap_or_else(|_| "http://chrome:4444".into());

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN env var is required"))?;

        let telegram_chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_CHAT_ID env var is required"))?;

        let check_interval_secs: u64 = std::env::var("CHECK_INTERVAL_SECS")
            .unwrap_or_else(|_| "60".into())
            .parse()
            .map_err(|_| anyhow::anyhow!("CHECK_INTERVAL_SECS must be a positive integer"))?;

        let data_dir = std::env::var("DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data"));

        let state_file = std::env::var("STATE_FILE").map(PathBuf::from).unwrap_or_else(|_| {
            let slug: String = monitor_target
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
                .collect();
            data_dir.join(format!("{slug}.state"))
        });

        Ok(Self {
            monitor_target,
            source_type,
            html_selector,
            webdriver_url,
            telegram_bot_token,
            telegram_chat_id,
            check_interval: Duration::from_secs(check_interval_secs),
            state_file,
            data_dir,
        })
    }
}
