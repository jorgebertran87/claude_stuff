use reqwest::Client;
use serde_json::json;

pub struct TelegramNotifier {
    client: Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            client: Client::new(),
            bot_token,
            chat_id,
        }
    }

    /// Send a change notification.
    /// `location` is the file path (shown to the user as the monitored source).
    /// `diff` is the textual diff produced by `watcher::compute_diff`.
    pub async fn send_change_notification(
        &self,
        location: &str,
        diff: &str,
    ) -> anyhow::Result<()> {
        // HTML parse mode is safer than MarkdownV2 — fewer edge-cases when
        // diff content contains special characters.
        let message = format!(
            "🔔 <b>File change detected</b>\n\n\
             📄 <b>File:</b> <code>{}</code>\n\n\
             <b>Diff:</b>\n<pre>{}</pre>",
            html_escape(location),
            html_escape(diff),
        );

        self.send_message(&message).await
    }

    // -----------------------------------------------------------------------
    // Private
    // -----------------------------------------------------------------------

    async fn send_message(&self, text: &str) -> anyhow::Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let payload = json!({
            "chat_id":   self.chat_id,
            "text":      text,
            "parse_mode": "HTML",
            // Disable link previews to avoid noisy previews for file paths.
            "link_preview_options": { "is_disabled": true },
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request to Telegram failed: {}", e))?;

        if resp.status().is_success() {
            return Ok(());
        }

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Telegram API returned {}: {}", status, body);
    }
}

// ---------------------------------------------------------------------------
// HTML escaping (only the three characters required by Telegram HTML mode)
// ---------------------------------------------------------------------------

fn html_escape(s: &str) -> String {
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
