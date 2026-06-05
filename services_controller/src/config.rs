use std::path::PathBuf;

pub struct Config {
    /// Path to the YAML file declaring alias → service mappings.
    /// Default: `/config/aliases.yaml` (mount it there in the container).
    pub alias_config: PathBuf,
    /// Docker Engine HTTP API endpoint used by `DockerController`.
    /// Default: `http://localhost:2375` (a TCP proxy in front of the socket).
    pub docker_api_url: String,
    /// Telegram bot token (from BotFather). Required only for `bot` mode.
    pub telegram_bot_token: Option<String>,
    /// Chats allowed to control services via Telegram.
    /// Empty means "allow any chat".
    pub telegram_allowed_chats: Vec<i64>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let alias_config = std::env::var("ALIAS_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/config/aliases.yaml"));

        let docker_api_url = std::env::var("DOCKER_API_URL")
            .unwrap_or_else(|_| "http://localhost:2375".into());

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();

        // Comma-separated chat IDs; blank/missing leaves the allow-list empty.
        let telegram_allowed_chats = std::env::var("TELEGRAM_ALLOWED_CHATS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();

        Ok(Self {
            alias_config,
            docker_api_url,
            telegram_bot_token,
            telegram_allowed_chats,
        })
    }
}
