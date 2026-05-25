use std::{path::PathBuf, time::Duration};

pub struct Config {
    /// Passed to the concrete `Source` implementation chosen at startup.
    /// A value that starts with `http://` or `https://` selects `HttpSource`;
    /// anything else selects `FileSource`.
    pub monitor_target: String,
    /// Optional CSS selector applied when using `HttpSource`.
    /// Example: `a[id="237"]`
    pub html_selector: Option<String>,
    /// Telegram Bot token (from BotFather).
    pub telegram_bot_token: String,
    /// Telegram chat/channel ID.
    pub telegram_chat_id: String,
    /// How often to poll the source.
    pub check_interval: Duration,
    /// Where the detector persists its last-known snapshot.
    pub state_file: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let monitor_target = std::env::var("MONITOR_TARGET")
            .map_err(|_| anyhow::anyhow!("MONITOR_TARGET env var is required"))?;

        let html_selector = std::env::var("HTML_SELECTOR").ok();

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN env var is required"))?;

        let telegram_chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_CHAT_ID env var is required"))?;

        let check_interval_secs: u64 = std::env::var("CHECK_INTERVAL_SECS")
            .unwrap_or_else(|_| "60".into())
            .parse()
            .map_err(|_| anyhow::anyhow!("CHECK_INTERVAL_SECS must be a positive integer"))?;

        // STATE_FILE defaults to /data/<slug>.state (persists across container
        // restarts when /data is a Docker volume).
        let state_file = std::env::var("STATE_FILE").map(PathBuf::from).unwrap_or_else(|_| {
            // Derive a safe filename from the target (replace path separators / `:` / `?`).
            let slug: String = monitor_target
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
                .collect();
            PathBuf::from(format!("/data/{slug}.state"))
        });

        Ok(Self {
            monitor_target,
            html_selector,
            telegram_bot_token,
            telegram_chat_id,
            check_interval: Duration::from_secs(check_interval_secs),
            state_file,
        })
    }
}
