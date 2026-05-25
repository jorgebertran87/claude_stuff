use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::{
    monitor::{MonitorConfig, MonitorMode, MonitorStore},
    runner::{MonitorSpawner, Notifier},
    source::Source,
};

// ---------------------------------------------------------------------------
// TelegramNotifier — sends change-detection alerts
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TelegramNotifier {
    client: reqwest::Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self { client: reqwest::Client::new(), bot_token, chat_id }
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

/// Implement the runner `Notifier` trait so `TelegramNotifier` can be passed
/// to `run_loop` without creating a circular dependency.
#[async_trait]
impl Notifier for TelegramNotifier {
    async fn notify(&self, location: &str, diff: &str) -> anyhow::Result<()> {
        self.send_change_notification(location, diff).await
    }
}

// ---------------------------------------------------------------------------
// CommandHandler — long-polls for bot commands and handles conversations
// ---------------------------------------------------------------------------

/// Multi-step conversation state for interactive commands.
#[derive(Clone, Debug)]
enum ConversationStep {
    // /add steps
    WaitingForAlias,
    WaitingForSelector { alias: String },
    WaitingForMode     { alias: String, selector: String },
    WaitingForInterval { alias: String, selector: String, mode: MonitorMode },
    // /remove step
    WaitingForRemoveTarget,
}

pub struct CommandHandler {
    client: reqwest::Client,
    bot_token: String,
    chat_id: i64,
    source: Arc<dyn Source>,
    interval_secs: u64,
    spawner: MonitorSpawner,
    store: Arc<Mutex<MonitorStore>>,
    /// Tracks an in-progress `/add` conversation (one at a time per chat).
    conversation: Arc<Mutex<Option<ConversationStep>>>,
}

impl CommandHandler {
    pub fn new(
        bot_token: String,
        chat_id_str: &str,
        source: Arc<dyn Source>,
        interval_secs: u64,
        spawner: MonitorSpawner,
        store: Arc<Mutex<MonitorStore>>,
    ) -> anyhow::Result<Self> {
        let chat_id: i64 = chat_id_str
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("TELEGRAM_CHAT_ID must be an integer, got '{chat_id_str}'"))?;
        Ok(Self {
            client: reqwest::Client::new(),
            bot_token,
            chat_id,
            source,
            interval_secs,
            spawner,
            store,
            conversation: Arc::new(Mutex::new(None)),
        })
    }

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
    // Message dispatch
    // -----------------------------------------------------------------------

    async fn handle_message(&self, msg: IncomingMessage) {
        if msg.chat.id != self.chat_id {
            return;
        }

        let text = msg.text.as_deref().unwrap_or("").trim();
        let is_command = text.starts_with('/');

        // Snapshot and clear conversation state without holding the lock over awaits.
        let step = {
            let mut guard = self.conversation.lock().await;
            if is_command {
                // Any command cancels an in-progress conversation.
                guard.take()
            } else {
                guard.clone()
            }
        };

        // If there is a pending conversation step and this is not a command,
        // route to the conversation handler.
        if !is_command {
            if let Some(step) = step {
                self.handle_conversation_reply(text, step).await;
                return;
            }
            // Plain message with no active conversation — ignore.
            return;
        }

        // Strip optional "@botname" suffix added in groups.
        let command = text.split('@').next().unwrap_or("").trim();
        match command {
            "/add"            => self.cmd_add().await,
            "/remove"         => self.cmd_remove().await,
            "/status" | "/check" => self.cmd_status().await,
            "/cancel"         => {
                let _ = send_message(
                    &self.client, &self.bot_token, &self.chat_id.to_string(),
                    "❌ Cancelled.",
                ).await;
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // /add — multi-step conversation
    // -----------------------------------------------------------------------

    async fn cmd_add(&self) {
        {
            let mut guard = self.conversation.lock().await;
            *guard = Some(ConversationStep::WaitingForAlias);
        }
        let _ = send_message(
            &self.client, &self.bot_token, &self.chat_id.to_string(),
            "➕ <b>New monitor</b>\n\n\
             Step 1/4 — Send the <b>alias</b> for this monitor:\n\
             <i>A short name to identify it, e.g. </i><code>match-456</code>",
        ).await;
    }

    // -----------------------------------------------------------------------
    // /remove — list monitors, ask which to stop
    // -----------------------------------------------------------------------

    async fn cmd_remove(&self) {
        let chat = self.chat_id.to_string();
        let aliases = self.spawner.list_aliases().await;

        if aliases.is_empty() {
            let _ = send_message(
                &self.client, &self.bot_token, &chat,
                "ℹ️ No monitors are currently running.",
            ).await;
            return;
        }

        let list = aliases
            .iter()
            .map(|a| format!("  • <code>{}</code>", html_escape(a)))
            .collect::<Vec<_>>()
            .join("\n");

        {
            let mut guard = self.conversation.lock().await;
            *guard = Some(ConversationStep::WaitingForRemoveTarget);
        }

        let _ = send_message(
            &self.client, &self.bot_token, &chat,
            &format!(
                "🗑 <b>Remove monitor</b>\n\n\
                 Running monitors:\n{list}\n\n\
                 Reply with the <b>alias</b> to stop and remove:\n\
                 <i>(Send /cancel to abort)</i>",
            ),
        ).await;
    }

    async fn handle_conversation_reply(&self, text: &str, step: ConversationStep) {
        let chat = self.chat_id.to_string();
        match step {
            // ── /remove: alias received ────────────────────────────────────
            ConversationStep::WaitingForRemoveTarget => {
                let alias = text.trim().to_string();

                // Abort the task.
                let stopped = self.spawner.remove(&alias).await;

                // Remove from persistent store (no-op for "main").
                let store_result = if alias != "main" {
                    let mut guard = self.store.lock().await;
                    guard.remove(&alias)
                } else {
                    Ok(false)
                };

                let reply = if stopped {
                    match store_result {
                        Ok(_) => format!(
                            "✅ Monitor <code>{}</code> stopped and removed.",
                            html_escape(&alias),
                        ),
                        Err(e) => format!(
                            "⚠️ Monitor <code>{}</code> stopped but could not be removed \
                             from storage: {}",
                            html_escape(&alias),
                            html_escape(&e.to_string()),
                        ),
                    }
                } else {
                    format!(
                        "❌ No running monitor named <code>{}</code>. \
                         Send /remove to see the current list.",
                        html_escape(&alias),
                    )
                };

                let _ = send_message(&self.client, &self.bot_token, &chat, &reply).await;
            }

            // ── Step 1: alias received ─────────────────────────────────────
            ConversationStep::WaitingForAlias => {
                let alias = text.to_string();
                {
                    let mut guard = self.conversation.lock().await;
                    *guard = Some(ConversationStep::WaitingForSelector { alias: alias.clone() });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ Alias: <code>{}</code>\n\n\
                         Step 2/4 — Send the <b>CSS selector</b> to monitor:\n\
                         <i>e.g. </i><code>[id=\"456\"] .some-class</code>",
                        html_escape(&alias),
                    ),
                ).await;
            }

            // ── Step 2: selector received ──────────────────────────────────
            ConversationStep::WaitingForSelector { alias } => {
                let selector = text.to_string();
                {
                    let mut guard = self.conversation.lock().await;
                    *guard = Some(ConversationStep::WaitingForMode {
                        alias: alias.clone(),
                        selector: selector.clone(),
                    });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ Selector: <code>{}</code>\n\n\
                         Step 3/4 — What should I monitor?\n\n\
                         • <code>content</code> — notify when the element's HTML changes\n\
                         • <code>exists</code>  — notify when the element appears or disappears",
                        html_escape(&selector),
                    ),
                ).await;
            }

            // ── Step 3: mode received ──────────────────────────────────────
            ConversationStep::WaitingForMode { alias, selector } => {
                let mode = match text.trim().to_lowercase().as_str() {
                    "content" | "c"           => MonitorMode::Content,
                    "exists"  | "existence" | "e" => MonitorMode::Existence,
                    _ => {
                        // Invalid — keep the same step and ask again.
                        {
                            let mut guard = self.conversation.lock().await;
                            *guard = Some(ConversationStep::WaitingForMode { alias, selector });
                        }
                        let _ = send_message(
                            &self.client, &self.bot_token, &chat,
                            "❌ Please reply with <code>content</code> or <code>exists</code>:",
                        ).await;
                        return;
                    }
                };
                let mode_label = match mode {
                    MonitorMode::Content   => "content (HTML changes)",
                    MonitorMode::Existence => "exists (appears / disappears)",
                };
                {
                    let mut guard = self.conversation.lock().await;
                    *guard = Some(ConversationStep::WaitingForInterval {
                        alias: alias.clone(),
                        selector: selector.clone(),
                        mode,
                    });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ Mode: <b>{mode_label}</b>\n\n\
                         Step 4/4 — Send the <b>check interval</b> in seconds:\n\
                         <i>e.g. </i><code>60</code>",
                    ),
                ).await;
            }

            // ── Step 4: interval received ──────────────────────────────────
            ConversationStep::WaitingForInterval { alias, selector, mode } => {
                let interval_secs = match text.trim().parse::<u64>() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        // Invalid — keep the same step and ask again.
                        {
                            let mut guard = self.conversation.lock().await;
                            *guard = Some(ConversationStep::WaitingForInterval {
                                alias,
                                selector,
                                mode,
                            });
                        }
                        let _ = send_message(
                            &self.client, &self.bot_token, &chat,
                            "❌ Please send a positive integer (number of seconds):",
                        ).await;
                        return;
                    }
                };

                // Conversation complete — clear state before any await.
                {
                    let mut guard = self.conversation.lock().await;
                    *guard = None;
                }

                let mode_label = match mode {
                    MonitorMode::Content   => "content",
                    MonitorMode::Existence => "exists",
                };
                let config = MonitorConfig {
                    alias: alias.clone(),
                    selector: selector.clone(),
                    interval_secs,
                    mode,
                };

                // Persist and spawn.
                let save_result = {
                    let mut guard = self.store.lock().await;
                    guard.add(config.clone())
                };

                match save_result {
                    Ok(()) => {
                        self.spawner.spawn(config).await;
                        let _ = send_message(
                            &self.client, &self.bot_token, &chat,
                            &format!(
                                "✅ <b>Monitor added and running!</b>\n\n\
                                 🏷 <b>Alias:</b>    <code>{}</code>\n\
                                 🔍 <b>Selector:</b> <code>{}</code>\n\
                                 👁 <b>Mode:</b>     {}\n\
                                 ⏱ <b>Interval:</b> {} s\n\n\
                                 <i>Send /cancel at any time to abort a new /add.</i>",
                                html_escape(&alias),
                                html_escape(&selector),
                                mode_label,
                                interval_secs,
                            ),
                        ).await;
                    }
                    Err(e) => {
                        error!("Failed to persist monitor '{alias}': {e}");
                        let _ = send_message(
                            &self.client, &self.bot_token, &chat,
                            &format!("❌ Failed to save monitor: {}", html_escape(&e.to_string())),
                        ).await;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // /status — show bot state + live content
    // -----------------------------------------------------------------------

    async fn cmd_status(&self) {
        info!("Received /status from chat {}", self.chat_id);
        let chat = self.chat_id.to_string();

        let placeholder_id = match send_message(
            &self.client, &self.bot_token, &chat,
            "🔄 Fetching current content…",
        ).await {
            Ok(id) => id,
            Err(e) => { error!("Failed to send status placeholder: {e}"); return; }
        };

        let content_line = match self.source.fetch().await {
            Ok(text) => format!(
                "\n\n🔍 <b>Current content:</b>\n<code>{}</code>",
                html_escape(text.trim()),
            ),
            Err(e) => {
                warn!("Status fetch failed: {e}");
                format!(
                    "\n\n🔍 <b>Current content:</b> <i>fetch failed — {}</i>",
                    html_escape(&e.to_string()),
                )
            }
        };

        // List dynamic monitors.
        let monitors_line = {
            let guard = self.store.lock().await;
            let all = guard.all();
            if all.is_empty() {
                String::new()
            } else {
                let list = all
                    .iter()
                    .map(|m| {
                        let mode_icon = match m.mode {
                            MonitorMode::Content   => "📝",
                            MonitorMode::Existence => "👁",
                        };
                        format!(
                            "  • <code>{}</code> — <code>{}</code> {} every {} s",
                            html_escape(&m.alias),
                            html_escape(&m.selector),
                            mode_icon,
                            m.interval_secs,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("\n\n📋 <b>Extra monitors:</b>\n{list}")
            }
        };

        let reply = format!(
            "✅ <b>Bot is running</b>\n\n\
             📄 <b>Monitoring:</b> <code>{}</code>\n\
             ⏱ <b>Check interval:</b> {} s\
             {content_line}\
             {monitors_line}",
            html_escape(self.source.location()),
            self.interval_secs,
        );

        if let Err(e) = edit_message(&self.client, &self.bot_token, &chat, placeholder_id, &reply).await {
            error!("Failed to edit status reply: {e}");
        }
    }

    // -----------------------------------------------------------------------
    // Polling
    // -----------------------------------------------------------------------

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
            .timeout(Duration::from_secs(40))
            .send()
            .await?
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
}

// ---------------------------------------------------------------------------
// Shared HTTP helpers
// ---------------------------------------------------------------------------

async fn send_message(
    client: &reqwest::Client,
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
    let resp = client.post(&url).json(&payload).send().await
        .map_err(|e| anyhow::anyhow!("sendMessage request failed: {e}"))?;

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

async fn edit_message(
    client: &reqwest::Client,
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
    let resp = client.post(&url).json(&payload).send().await
        .map_err(|e| anyhow::anyhow!("editMessageText request failed: {e}"))?;

    if resp.status().is_success() { return Ok(()); }
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
// Telegram API DTOs
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
