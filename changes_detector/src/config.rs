use std::path::PathBuf;

pub struct Config {
    /// Telegram Bot token (from BotFather).
    pub telegram_bot_token: String,
    /// Telegram chat/channel ID.
    pub telegram_chat_id: String,
    /// WebDriver server URL used by `BrowserSource` for every monitor.
    /// Default: `http://chrome:4444` (the service name in docker-compose).
    pub webdriver_url: String,
    /// Directory used for all state files and the monitor store.
    pub data_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN env var is required"))?;

        let telegram_chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_CHAT_ID env var is required"))?;

        let webdriver_url = std::env::var("WEBDRIVER_URL")
            .unwrap_or_else(|_| "http://chrome:4444".into());

        let data_dir = std::env::var("DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data"));

        Ok(Self {
            telegram_bot_token,
            telegram_chat_id,
            webdriver_url,
            data_dir,
        })
    }
}
