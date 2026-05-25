use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info, warn};

use crate::source::Source;

// ---------------------------------------------------------------------------
// Notifier — sends change-detection alerts to a Telegram chat
// ---------------------------------------------------------------------------

pub struct TelegramNotifier {
    client: Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self { client: Client::new(), bot_token, chat_id }
    }

    pub async fn send_change_notification(
        &self,
        location: &str,
        diff: &str,
    ) -> anyhow::Result<()> {
        let message = format!(
            "🔔 <b>Change detected</b>\n\n\
             📄 <b>Source:</b> <code>{}</code>\n\n\
             <b>Diff:</b>\n<pre>{}</pre>",
            html_escape(location),
            html_escape(diff),
        );
        send_message(&self.client, &self.bot_token, &self.chat_id, &message).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CommandHandler — long-polls for bot commands and replies to them
// ---------------------------------------------------------------------------

/// Listens for Telegram commands and responds to them.
///
/// Runs forever in its own tokio task (`tokio::spawn(handler.run())`).
/// Only reacts to messages from the configured chat so strangers cannot
/// query the bot.
///
/// Supported commands:
///   /status   — confirms the bot is alive and shows what is being monitored
///   /check    — alias for /status
pub struct CommandHandler {
    client: Client,
    bot_token: String,
    /// The authorised chat id (as configured in TELEGRAM_CHAT_ID).
    chat_id: i64,
    /// The source is fetched live when /status is requested.
    source: Arc<dyn Source>,
    interval_secs: u64,
}

impl CommandHandler {
    /// `chat_id_str` is the raw TELEGRAM_CHAT_ID string (e.g. `"-1001234567890"`).
    pub fn new(
        bot_token: String,
        chat_id_str: &str,
        source: Arc<dyn Source>,
        interval_secs: u64,
    ) -> anyhow::Result<Self> {
        let chat_id: i64 = chat_id_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("TELEGRAM_CHAT_ID must be a valid integer, got '{chat_id_str}'"))?;

        Ok(Self {
            client: Client::new(),
            bot_token,
            chat_id,
            source,
            interval_secs,
        })
    }

    /// Entry point — call with `tokio::spawn(handler.run())`.
    pub async fn run(self) {
        info!("Telegram command handler started (chat_id={})", self.chat_id);
        let mut offset: i64 = 0;

        loop {
            match self.poll_updates(offset).await {
                Ok(updates) => {
                    for update in updates {
                        offset = update.update_id + 1;
                        if let Some(msg) = update.message {
                            self.handle_message(msg).await;
                        }
                    }
                }
                Err(e) => {
                    warn!("getUpdates failed: {e} — retrying in 5 s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private
    // -----------------------------------------------------------------------

    /// Long-poll the Telegram Bot API for new updates.
    /// Blocks for up to 30 s if there is nothing to process.
    async fn poll_updates(&self, offset: i64) -> anyhow::Result<Vec<Update>> {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", self.bot_token);

        let resp = self
            .client
            .get(&url)
            .query(&[
                ("offset",          offset.to_string()),
                ("timeout",         "30".to_string()),
                ("allowed_updates", r#"["message"]"#.to_string()),
            ])
            // Must be > poll timeout to avoid a spurious reqwest timeout.
            .timeout(Duration::from_secs(40))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let updates = resp["result"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<Update>(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(updates)
    }

    async fn handle_message(&self, msg: IncomingMessage) {
        // Ignore messages from other chats.
        if msg.chat.id != self.chat_id {
            return;
        }

        let text = msg.text.as_deref().unwrap_or("");

        // Strip optional "@botname" suffix that Telegram appends in groups.
        let command = text.split('@').next().unwrap_or("").trim();

        match command {
            "/status" | "/check" => {
                info!("Received {command} from chat {}", self.chat_id);
                let chat_id_str = self.chat_id.to_string();

                // Reply immediately so the user knows the command was received.
                let placeholder_id = match send_message(
                    &self.client,
                    &self.bot_token,
                    &chat_id_str,
                    "🔄 Fetching current content…",
                )
                .await
                {
                    Ok(id) => id,
                    Err(e) => { error!("Failed to send placeholder for {command}: {e}"); return; }
                };

                // Now do the (potentially slow) source fetch.
                let content_line = match self.source.fetch().await {
                    Ok(text) => format!(
                        "\n\n🔍 <b>Current content:</b>\n<code>{}</code>",
                        html_escape(text.trim()),
                    ),
                    Err(e) => {
                        warn!("Could not fetch source for {command} reply: {e}");
                        format!(
                            "\n\n🔍 <b>Current content:</b> <i>fetch failed — {}</i>",
                            html_escape(&e.to_string()),
                        )
                    }
                };

                let reply = format!(
                    "✅ <b>Bot is running</b>\n\n\
                     📄 <b>Monitoring:</b> <code>{}</code>\n\
                     ⏱ <b>Check interval:</b> {} s\
                     {content_line}",
                    html_escape(self.source.location()),
                    self.interval_secs,
                );

                // Edit the placeholder in-place with the final reply.
                if let Err(e) = edit_message(
                    &self.client,
                    &self.bot_token,
                    &chat_id_str,
                    placeholder_id,
                    &reply,
                )
                .await
                {
                    error!("Failed to edit status reply: {e}");
                }
            }
            _ => {} // Ignore unknown commands and plain messages.
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Send a message and return the `message_id` assigned by Telegram.
async fn send_message(
    client: &Client,
    bot_token: &str,
    chat_id: &str,
    text: &str,
) -> anyhow::Result<i64> {
    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

    let payload = json!({
        "chat_id":    chat_id,
        "text":       text,
        "parse_mode": "HTML",
        "link_preview_options": { "is_disabled": true },
    });

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP request to Telegram failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Telegram API returned {status}: {body}");
    }

    let body: serde_json::Value = resp.json().await
        .map_err(|e| anyhow::anyhow!("Failed to parse sendMessage response: {e}"))?;

    body["result"]["message_id"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("sendMessage response missing message_id"))
}

/// Edit the text of an already-sent message in-place.
async fn edit_message(
    client: &Client,
    bot_token: &str,
    chat_id: &str,
    message_id: i64,
    text: &str,
) -> anyhow::Result<()> {
    let url = format!("https://api.telegram.org/bot{bot_token}/editMessageText");

    let payload = json!({
        "chat_id":    chat_id,
        "message_id": message_id,
        "text":       text,
        "parse_mode": "HTML",
        "link_preview_options": { "is_disabled": true },
    });

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP request to Telegram failed: {e}"))?;

    if resp.status().is_success() {
        return Ok(());
    }

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    anyhow::bail!("editMessageText returned {status}: {body}");
}

pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Telegram API DTOs (private — only used for update deserialization)
// ---------------------------------------------------------------------------

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
