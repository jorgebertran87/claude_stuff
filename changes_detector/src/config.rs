use std::{path::PathBuf, time::Duration};

pub struct Config {
    /// Path to the file being monitored.
    pub monitor_file: PathBuf,
    /// Telegram Bot token (from BotFather).
    pub telegram_bot_token: String,
    /// Telegram chat/channel ID where notifications are sent.
    pub telegram_chat_id: String,
    /// How often to poll for changes.
    pub check_interval: Duration,
    /// Where to persist the last-known file content.
    pub state_file: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let monitor_file = PathBuf::from(
            std::env::var("MONITOR_FILE")
                .map_err(|_| anyhow::anyhow!("MONITOR_FILE env var is required"))?,
        );

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN env var is required"))?;

        let telegram_chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_CHAT_ID env var is required"))?;

        let check_interval_secs: u64 = std::env::var("CHECK_INTERVAL_SECS")
            .unwrap_or_else(|_| "60".into())
            .parse()
            .map_err(|_| anyhow::anyhow!("CHECK_INTERVAL_SECS must be a positive integer"))?;

        // State file defaults to /data/<filename>.state so it survives container restarts
        // when /data is a Docker volume. Can be overridden via STATE_FILE.
        let state_file = std::env::var("STATE_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let stem = monitor_file
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                PathBuf::from(format!("/data/{stem}.state"))
            });

        Ok(Self {
            monitor_file,
            telegram_bot_token,
            telegram_chat_id,
            check_interval: Duration::from_secs(check_interval_secs),
            state_file,
        })
    }
}
