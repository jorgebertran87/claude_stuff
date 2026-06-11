use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use super::{TelegramGateway, TelegramUpdate};

const DEFAULT_API_BASE: &str = "https://api.telegram.org";

/// How long the server holds a `getUpdates` request open waiting for a message.
const LONG_POLL_SECS: u64 = 30;

/// Production [`TelegramGateway`] backed by the Telegram Bot API over HTTP.
///
/// Polling is resilient: any transport, HTTP, or parse error yields no updates
/// rather than propagating, so the bot's poll loop simply tries again.
pub struct HttpTelegramGateway {
    client: reqwest::Client,
    base_url: String,
    bot_token: String,
}

impl HttpTelegramGateway {
    pub fn new(bot_token: String) -> Self {
        Self::with_base_url(DEFAULT_API_BASE.to_string(), bot_token)
    }

    /// Build a gateway against a specific API base URL — used by tests to point
    /// at a mock server.
    pub fn with_base_url(base_url: String, bot_token: String) -> Self {
        Self { client: reqwest::Client::new(), base_url, bot_token }
    }

    fn method_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.base_url, self.bot_token, method)
    }

    /// Long-poll `getUpdates`, surfacing any transport, HTTP, or parse error.
    async fn try_fetch_updates(&self, offset: i64) -> reqwest::Result<Vec<TelegramUpdate>> {
        let body: GetUpdatesResponse = self
            .client
            .get(self.method_url("getUpdates"))
            .query(&[
                ("offset", offset.to_string()),
                ("timeout", LONG_POLL_SECS.to_string()),
                ("allowed_updates", r#"["message"]"#.to_string()),
            ])
            .timeout(Duration::from_secs(LONG_POLL_SECS + 10))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(body
            .result
            .into_iter()
            .filter_map(|u| {
                let msg = u.message?;
                let text = msg.text?;
                Some(TelegramUpdate {
                    update_id: u.update_id,
                    chat_id: msg.chat.id,
                    text,
                    date: msg.date,
                })
            })
            .collect())
    }
}

// ── Telegram API DTOs (only the fields we need) ──────────────────────────────

#[derive(Deserialize)]
struct GetUpdatesResponse {
    result: Vec<RawUpdate>,
}

#[derive(Deserialize)]
struct RawUpdate {
    update_id: i64,
    message: Option<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    chat: RawChat,
    text: Option<String>,
    // Defaulting a missing date to 0 makes the update maximally stale, so the
    // bot confirms it without executing — fail closed.
    #[serde(default)]
    date: i64,
}

#[derive(Deserialize)]
struct RawChat {
    id: i64,
}

#[async_trait]
impl TelegramGateway for HttpTelegramGateway {
    async fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate> {
        self.try_fetch_updates(offset).await.unwrap_or_else(|e| {
            // The request URL embeds the bot token — strip it before logging.
            warn!("getUpdates failed: {}", e.without_url());
            Vec::new()
        })
    }

    async fn post_message(&self, chat_id: i64, text: &str) {
        let payload = json!({ "chat_id": chat_id, "text": text });
        let result = self
            .client
            .post(self.method_url("sendMessage"))
            .json(&payload)
            .send()
            .await
            .and_then(|r| r.error_for_status());
        if let Err(e) = result {
            warn!("sendMessage to chat {chat_id} failed: {}", e.without_url());
        }
    }
}
