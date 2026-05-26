use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::{
    monitor::{MonitorConfig, MonitorMode, MonitorStore, SourceType},
    runner::{MonitorSpawner, Notifier},
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
        // location is "https://…  [selector]" — extract just the URL for the link.
        let url = location.split("  [").next().unwrap_or(location).trim();
        let message = format!(
            "🔔 <b>Change detected</b>\n\n\
             📄 <b>Source:</b> <code>{}</code>\n\
             🔗 <a href=\"{}\">Open page</a>\n\n\
             <b>Diff:</b>\n<pre>{}</pre>",
            html_escape(location),
            html_escape(url),
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
    // /add steps (6 steps total)
    WaitingForAlias,
    WaitingForUrl        { alias: String },
    WaitingForSourceType { alias: String, url: String },
    WaitingForSelector   { alias: String, url: String, source_type: SourceType },
    WaitingForMode       { alias: String, url: String, source_type: SourceType, selector: String },
    WaitingForInterval   { alias: String, url: String, source_type: SourceType, selector: String, mode: MonitorMode },
    // /remove step
    WaitingForRemoveTarget,
}

pub struct CommandHandler {
    client: reqwest::Client,
    bot_token: String,
    chat_id: i64,
    spawner: MonitorSpawner,
    store: Arc<Mutex<MonitorStore>>,
    /// Tracks an in-progress `/add` conversation (one at a time per chat).
    conversation: Arc<Mutex<Option<ConversationStep>>>,
}

impl CommandHandler {
    pub fn new(
        bot_token: String,
        chat_id_str: &str,
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

        // Strip optional "@botname" suffix added in groups, then split
        // the command word from any trailing argument (e.g. "/check alias").
        let full = text.split('@').next().unwrap_or("").trim();
        let (cmd_word, cmd_arg) = match full.split_once(' ') {
            Some((w, a)) => (w.trim(), a.trim()),
            None         => (full, ""),
        };

        match cmd_word {
            "/add"    => self.cmd_add().await,
            "/remove" => self.cmd_remove().await,
            "/status" => self.cmd_status().await,
            "/check"  => self.cmd_check_content(cmd_arg).await,
            "/cancel" => {
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
             Step 1/6 — Send the <b>alias</b> for this monitor:\n\
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
                    *guard = Some(ConversationStep::WaitingForUrl { alias: alias.clone() });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ Alias: <code>{}</code>\n\n\
                         Step 2/6 — Send the <b>URL</b> to monitor:\n\
                         <i>e.g. </i><code>https://example.com/page</code>",
                        html_escape(&alias),
                    ),
                ).await;
            }

            // ── Step 2: URL received ───────────────────────────────────────
            ConversationStep::WaitingForUrl { alias } => {
                let url = text.trim().to_string();
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    {
                        let mut guard = self.conversation.lock().await;
                        *guard = Some(ConversationStep::WaitingForUrl { alias });
                    }
                    let _ = send_message(
                        &self.client, &self.bot_token, &chat,
                        "❌ URL must start with <code>http://</code> or <code>https://</code>.\n\
                         Please send a valid URL:",
                    ).await;
                    return;
                }
                {
                    let mut guard = self.conversation.lock().await;
                    *guard = Some(ConversationStep::WaitingForSourceType {
                        alias: alias.clone(),
                        url: url.clone(),
                    });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ URL: <code>{}</code>\n\n\
                         Step 3/6 — Which fetch method?\n\n\
                         • <code>browser</code> — headless Chrome (most sites)\n\
                         • <code>flare</code>   — FlareSolverr (Cloudflare-protected sites)",
                        html_escape(&url),
                    ),
                ).await;
            }

            // ── Step 3: source type received ───────────────────────────────
            ConversationStep::WaitingForSourceType { alias, url } => {
                let source_type = match text.trim().to_lowercase().as_str() {
                    "browser" | "b" => SourceType::Browser,
                    "flare"   | "f" => SourceType::Flare,
                    _ => {
                        {
                            let mut guard = self.conversation.lock().await;
                            *guard = Some(ConversationStep::WaitingForSourceType { alias, url });
                        }
                        let _ = send_message(
                            &self.client, &self.bot_token, &chat,
                            "❌ Please reply with <code>browser</code> or <code>flare</code>:",
                        ).await;
                        return;
                    }
                };
                let source_label = match source_type {
                    SourceType::Browser => "browser (headless Chrome)",
                    SourceType::Flare   => "flare (FlareSolverr)",
                };
                {
                    let mut guard = self.conversation.lock().await;
                    *guard = Some(ConversationStep::WaitingForSelector {
                        alias: alias.clone(),
                        url: url.clone(),
                        source_type,
                    });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ Method: <b>{source_label}</b>\n\n\
                         Step 4/6 — Send the <b>CSS selector</b> to monitor:\n\
                         <i>e.g. </i><code>[data-t=\"threadLink\"]</code>",
                    ),
                ).await;
            }

            // ── Step 4: selector received ──────────────────────────────────
            ConversationStep::WaitingForSelector { alias, url, source_type } => {
                let selector = text.to_string();
                {
                    let mut guard = self.conversation.lock().await;
                    *guard = Some(ConversationStep::WaitingForMode {
                        alias: alias.clone(),
                        url: url.clone(),
                        source_type,
                        selector: selector.clone(),
                    });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ Selector: <code>{}</code>\n\n\
                         Step 5/6 — What should I monitor?\n\n\
                         • <code>content</code> — notify when the element's HTML changes\n\
                         • <code>exists</code>  — notify when the element appears or disappears",
                        html_escape(&selector),
                    ),
                ).await;
            }

            // ── Step 5: mode received ──────────────────────────────────────
            ConversationStep::WaitingForMode { alias, url, source_type, selector } => {
                let mode = match text.trim().to_lowercase().as_str() {
                    "content" | "c"               => MonitorMode::Content,
                    "exists"  | "existence" | "e" => MonitorMode::Existence,
                    _ => {
                        {
                            let mut guard = self.conversation.lock().await;
                            *guard = Some(ConversationStep::WaitingForMode {
                                alias, url, source_type, selector,
                            });
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
                        url: url.clone(),
                        source_type,
                        selector: selector.clone(),
                        mode,
                    });
                }
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "✅ Mode: <b>{mode_label}</b>\n\n\
                         Step 6/6 — Send the <b>check interval</b> in seconds:\n\
                         <i>e.g. </i><code>60</code>",
                    ),
                ).await;
            }

            // ── Step 6: interval received ──────────────────────────────────
            ConversationStep::WaitingForInterval { alias, url, source_type, selector, mode } => {
                let interval_secs = match text.trim().parse::<u64>() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        {
                            let mut guard = self.conversation.lock().await;
                            *guard = Some(ConversationStep::WaitingForInterval {
                                alias,
                                url,
                                source_type,
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
                let source_label = match source_type {
                    SourceType::Browser => "browser",
                    SourceType::Flare   => "flare",
                };
                let config = MonitorConfig {
                    alias: alias.clone(),
                    url: Some(url.clone()),
                    selector: selector.clone(),
                    interval_secs,
                    mode,
                    source_type,
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
                                 🌐 <b>URL:</b>      <code>{}</code>\n\
                                 ⚙️ <b>Method:</b>   {}\n\
                                 🔍 <b>Selector:</b> <code>{}</code>\n\
                                 👁 <b>Mode:</b>     {}\n\
                                 ⏱ <b>Interval:</b> {} s\n\n\
                                 <i>Send /cancel at any time to abort a new /add.</i>",
                                html_escape(&alias),
                                html_escape(&url),
                                source_label,
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
    // /status — list all configured monitors
    // -----------------------------------------------------------------------

    async fn cmd_status(&self) {
        info!("Received /status from chat {}", self.chat_id);
        let chat = self.chat_id.to_string();

        let guard = self.store.lock().await;
        let all = guard.all();

        let reply = if all.is_empty() {
            "✅ <b>Bot is running</b>\n\n\
             ℹ️ No monitors yet — use /add to create one."
                .to_string()
        } else {
            let list = all
                .iter()
                .map(|m| {
                    let mode_label = match m.mode {
                        MonitorMode::Content   => "content 📝",
                        MonitorMode::Existence => "exists 👁",
                    };
                    let source_label = match m.source_type {
                        SourceType::Browser => "browser 🌐",
                        SourceType::Flare   => "flare 🔥",
                    };
                    let url_display = m.url.as_deref().unwrap_or("(no URL)");
                    let url_line = match &m.url {
                        Some(u) => format!(
                            "🌐 <a href=\"{}\">{}</a>",
                            html_escape(u),
                            html_escape(u),
                        ),
                        None => format!("🌐 {}", html_escape(url_display)),
                    };
                    format!(
                        "🏷 <b>{}</b>\n\
                         {}\n\
                         ⚙️ {} · 👁 {} · ⏱ {} s\n\
                         🔍 <code>{}</code>",
                        html_escape(&m.alias),
                        url_line,
                        source_label,
                        mode_label,
                        m.interval_secs,
                        html_escape(&m.selector),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");

            format!(
                "✅ <b>Bot is running</b>\n\n\
                 📋 <b>Monitors ({}):</b>\n\n{list}",
                all.len(),
            )
        };

        // Drop the lock before any await.
        drop(guard);

        if let Err(e) = send_message(&self.client, &self.bot_token, &chat, &reply).await {
            error!("Failed to send status reply: {e}");
        }
    }

    // -----------------------------------------------------------------------
    // /check <alias> — fetch the current text content of a monitor's selector
    // -----------------------------------------------------------------------

    async fn cmd_check_content(&self, alias: &str) {
        let chat = self.chat_id.to_string();

        if alias.is_empty() {
            let _ = send_message(
                &self.client, &self.bot_token, &chat,
                "ℹ️ Usage: /check <code>&lt;alias&gt;</code>\n\
                 <i>Fetches the current text content of the selector for that monitor.</i>\n\n\
                 Use /status to see available aliases.",
            ).await;
            return;
        }

        // Look up the monitor config.
        let config = {
            let guard = self.store.lock().await;
            guard.all().iter().find(|m| m.alias == alias).cloned()
        };

        let config = match config {
            Some(c) => c,
            None => {
                let _ = send_message(
                    &self.client, &self.bot_token, &chat,
                    &format!(
                        "❌ No monitor named <code>{}</code>.\n\
                         Use /status to see available aliases.",
                        html_escape(alias),
                    ),
                ).await;
                return;
            }
        };

        // Send a placeholder while fetching (can take up to 60 s via FlareSolverr).
        let placeholder_id = match send_message(
            &self.client, &self.bot_token, &chat,
            &format!("🔄 Fetching <code>{}</code>…", html_escape(alias)),
        ).await {
            Ok(id) => id,
            Err(e) => { error!("Failed to send check placeholder: {e}"); return; }
        };

        let url_line = match &config.url {
            Some(u) => format!("🔗 <a href=\"{}\">{}</a>\n", html_escape(u), html_escape(u)),
            None    => String::new(),
        };

        let reply = match self.spawner.fetch_text(&config).await {
            Ok(text) if text.is_empty() => format!(
                "🔍 <b>{}</b>\n{}\n<i>(empty — selector matched but contained no text)</i>",
                html_escape(alias),
                url_line,
            ),
            Ok(text) => format!(
                "🔍 <b>{}</b>\n{}\n<code>{}</code>",
                html_escape(alias),
                url_line,
                html_escape(&text),
            ),
            Err(e) => {
                warn!("check_content fetch failed for '{alias}': {e}");
                format!(
                    "❌ Fetch failed for <code>{}</code>:\n{}<i>{}</i>",
                    html_escape(alias),
                    url_line,
                    html_escape(&e.to_string()),
                )
            }
        };

        if let Err(e) = edit_message(&self.client, &self.bot_token, &chat, placeholder_id, &reply).await {
            error!("Failed to edit check reply: {e}");
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
