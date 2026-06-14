use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::basket::{BasketSource, OrderNormalizer};
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
    normalizer: Box<dyn OrderNormalizer>,
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
        normalizer: Box<dyn OrderNormalizer>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url,
            bot_token,
            chat_id,
            stores,
            baskets,
            normalizer,
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
            let reply = reply_to(&self.stores, &self.baskets, self.normalizer.as_ref(), &text).await;
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

    /// Send a reply, split into Telegram-sized messages when it is too long.
    async fn send_message(&self, text: &str) -> anyhow::Result<()> {
        for chunk in split_message(text, MAX_MESSAGE_LEN) {
            self.send_chunk(&chunk).await?;
        }
        Ok(())
    }

    async fn send_chunk(&self, text: &str) -> anyhow::Result<()> {
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

/// Telegram rejects messages longer than 4096 characters; stay under it with a
/// margin so a per-character count never disagrees with Telegram's own.
const MAX_MESSAGE_LEN: usize = 4000;

/// Split a reply into chunks of at most `max` characters, breaking between item
/// blocks (blank-line separated) so a block is never cut across messages. A
/// block that alone exceeds `max` is hard-split by characters as a last resort.
fn split_message(text: &str, max: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for block in text.split("\n\n") {
        if block.chars().count() > max {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            chunks.extend(hard_split(block, max));
            continue;
        }
        let joined = current.chars().count() + 2 + block.chars().count();
        if !current.is_empty() && joined > max {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(block);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        chunks.push(String::new());
    }
    chunks
}

/// Break `text` into pieces of at most `max` characters, on character
/// boundaries.
fn hard_split(text: &str, max: usize) -> Vec<String> {
    text.chars()
        .collect::<Vec<_>>()
        .chunks(max)
        .map(|c| c.iter().collect())
        .collect()
}
