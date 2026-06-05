use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use super::{TelegramGateway, TelegramUpdate};

/// Production [`TelegramGateway`] backed by the Telegram Bot API over HTTP.
pub struct HttpTelegramGateway {
    client: reqwest::Client,
    bot_token: String,
}

impl HttpTelegramGateway {
    pub fn new(bot_token: String) -> Self {
        Self { client: reqwest::Client::new(), bot_token }
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
}

#[derive(Deserialize)]
struct RawChat {
    id: i64,
}

#[async_trait]
impl TelegramGateway for HttpTelegramGateway {
    async fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate> {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", self.bot_token);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("offset", offset.to_string()),
                ("timeout", "30".to_string()),
                ("allowed_updates", r#"["message"]"#.to_string()),
            ])
            .timeout(Duration::from_secs(40))
            .send()
            .await;

        let parsed = match resp {
            Ok(r) => r.json::<GetUpdatesResponse>().await,
            Err(e) => {
                warn!("getUpdates request failed: {e}");
                return Vec::new();
            }
        };

        match parsed {
            Ok(body) => body
                .result
                .into_iter()
                .filter_map(|u| {
                    let msg = u.message?;
                    Some(TelegramUpdate {
                        update_id: u.update_id,
                        chat_id: msg.chat.id,
                        text: msg.text.unwrap_or_default(),
                    })
                })
                .collect(),
            Err(e) => {
                warn!("Failed to parse getUpdates response: {e}");
                Vec::new()
            }
        }
    }

    async fn post_message(&self, chat_id: i64, text: &str) {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let payload = json!({ "chat_id": chat_id, "text": text });
        if let Err(e) = self.client.post(&url).json(&payload).send().await {
            warn!("sendMessage to chat {chat_id} failed: {e}");
        }
    }
}
