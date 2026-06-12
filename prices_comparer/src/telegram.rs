use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::basket::BasketSource;
use crate::bot::reply_to;
use crate::comparer::StoreSource;

/// Long-polls Telegram for basket messages and answers with the comparison.
/// Only messages from the configured chat are handled.
pub struct TelegramBot {
    client: reqwest::Client,
    api_url: String,
    bot_token: String,
    chat_id: i64,
    stores: Vec<Box<dyn StoreSource>>,
    baskets: Vec<Box<dyn BasketSource>>,
    offset: i64,
}

#[derive(Deserialize)]
struct Update {
    update_id: i64,
    message: Option<IncomingMessage>,
}

#[derive(Deserialize)]
struct IncomingMessage {
    chat: Chat,
    text: Option<String>,
}

#[derive(Deserialize)]
struct Chat {
    id: i64,
}

impl TelegramBot {
    /// `api_url` is `https://api.telegram.org` in production or a mock
    /// server in tests.
    pub fn new(
        api_url: String,
        bot_token: String,
        chat_id: i64,
        stores: Vec<Box<dyn StoreSource>>,
        baskets: Vec<Box<dyn BasketSource>>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url,
            bot_token,
            chat_id,
            stores,
            baskets,
            offset: 0,
        }
    }

    pub async fn run(mut self) {
        eprintln!("Telegram bot started (chat_id={})", self.chat_id);
        loop {
            if let Err(e) = self.run_once().await {
                eprintln!("Telegram poll failed: {e} — retrying in 5 s");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    /// Poll once and answer every basket message from the configured chat.
    pub async fn run_once(&mut self) -> anyhow::Result<()> {
        for update in self.poll_updates().await? {
            self.offset = update.update_id + 1;
            let Some(msg) = update.message else { continue };
            if msg.chat.id != self.chat_id {
                continue;
            }
            let text = msg.text.unwrap_or_default();
            let reply = reply_to(&self.stores, &self.baskets, &text).await;
            self.send_message(&reply).await?;
        }
        Ok(())
    }

    async fn poll_updates(&self) -> anyhow::Result<Vec<Update>> {
        let url = format!("{}/bot{}/getUpdates", self.api_url, self.bot_token);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("offset", self.offset.to_string()),
                ("timeout", "30".to_string()),
                ("allowed_updates", r#"["message"]"#.to_string()),
            ])
            .timeout(Duration::from_secs(40))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;

        Ok(resp["result"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<Update>(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn send_message(&self, text: &str) -> anyhow::Result<()> {
        let url = format!("{}/bot{}/sendMessage", self.api_url, self.bot_token);
        let payload = json!({
            "chat_id": self.chat_id,
            "text": text,
            "link_preview_options": { "is_disabled": true },
        });
        let resp = self.client.post(&url).json(&payload).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Telegram sendMessage returned {status}: {body}");
        }
        Ok(())
    }
}
