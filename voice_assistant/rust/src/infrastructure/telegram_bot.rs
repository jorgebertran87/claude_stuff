//! Telegram bot adapter for the voice assistant.
//! Provides long-polling access to Telegram messages and routes them through OrderHandler.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::domain::ports::OrderHandler;
use crate::infrastructure::speaker::synthesize_alexa_spotify;

/// A single Telegram update containing message text.
#[derive(Clone)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub chat_id: i64,
    pub text: String,
}

/// Injectable HTTP gateway for Telegram API calls.
/// Separated for testability (real impl uses ureq, tests use FakeGateway).
pub trait TelegramGateway: Send + Sync {
    fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate>;
    fn post_message(&self, chat_id: i64, text: &str);
    /// Send an audio file as a voice message (MP3 bytes).
    fn send_voice(&self, chat_id: i64, data: &[u8]);
}

/// Real Telegram gateway using ureq HTTP client.
struct UreqGateway {
    token: String,
}

impl UreqGateway {
    fn new(token: String) -> Self {
        Self { token }
    }

    fn base_url(&self) -> String {
        format!("https://api.telegram.org/bot{}", self.token)
    }

    fn split_message(text: &str, max_len: usize) -> Vec<&str> {
        let mut chunks = Vec::new();
        let mut start = 0;
        while start < text.len() {
            let end = (start + max_len).min(text.len());
            // Walk back to a char boundary
            let end = text[..end]
                .char_indices()
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(end);
            chunks.push(&text[start..end]);
            start = end;
        }
        chunks
    }
}

impl TelegramGateway for UreqGateway {
    fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate> {
        let url = format!(
            "{}/getUpdates?offset={}&timeout=30",
            self.base_url(),
            offset
        );
        let resp = match ureq::get(&url)
            .timeout(Duration::from_secs(40))
            .call()
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[telegram get_updates error: {e}]");
                return vec![];
            }
        };

        let body = match resp.into_string() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[telegram body read error: {e}]");
                return vec![];
            }
        };

        parse_updates(&body)
    }

    fn post_message(&self, chat_id: i64, text: &str) {
        // Telegram message limit is 4096 chars; split if needed
        for chunk in Self::split_message(text, 4096) {
            let url = format!("{}/sendMessage", self.base_url());
            let json_text = serde_json::to_string(chunk)
                .unwrap_or_else(|_| "\"error sending message\"".into());
            let body = format!(r#"{{"chat_id": {}, "text": {}}}"#, chat_id, json_text);

            if let Err(e) = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
            {
                eprintln!("[telegram send_message error: {e}]");
            }
        }
    }

    fn send_voice(&self, chat_id: i64, data: &[u8]) {
        let url = format!("{}/sendVoice", self.base_url());
        let boundary = "TelegramVoiceBoundary";

        let mut body: Vec<u8> = Vec::new();
        // chat_id field
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"chat_id\"\r\n\r\n");
        body.extend_from_slice(chat_id.to_string().as_bytes());
        body.extend_from_slice(b"\r\n");
        // voice field
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"voice\"; filename=\"voice.mp3\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: audio/mpeg\r\n\r\n");
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        if let Err(e) = ureq::post(&url)
            .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .send_bytes(&body)
        {
            eprintln!("[telegram send_voice error: {e}]");
        }
    }
}

fn play_audio_bytes(bytes: &[u8]) {
    let tmp = "/tmp/tts_telegram_play.mp3";
    if std::fs::write(tmp, bytes).is_err() {
        return;
    }
    let _ = Command::new("ffplay")
        .args(["-nodisp", "-autoexit", "-loglevel", "quiet", tmp])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Main Telegram bot orchestrator.
pub struct TelegramBot {
    gateway: Box<dyn TelegramGateway>,
    allowed_chat_ids: Vec<i64>,
}

impl TelegramBot {
    /// Create a new Telegram bot using the real ureq HTTP client.
    /// Reads allowed chat IDs from the TELEGRAM_ALLOWED_CHAT_IDS env var.
    pub fn new(token: String) -> Self {
        let allowed: Vec<i64> = std::env::var("TELEGRAM_ALLOWED_CHAT_IDS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();

        if allowed.is_empty() {
            eprintln!("[telegram bot initializing with token (hidden), allowed chats: all (no filter)]");
        } else {
            eprintln!("[telegram bot initializing with token (hidden), allowed chats: {:?}]", allowed);
        }

        Self {
            gateway: Box::new(UreqGateway::new(token)),
            allowed_chat_ids: allowed,
        }
    }

    /// Create a new bot with an injectable gateway (for testing).
    #[allow(dead_code)]
    pub fn with_injectable(
        gateway: Box<dyn TelegramGateway>,
        allowed_chat_ids: Vec<i64>,
    ) -> Self {
        Self {
            gateway,
            allowed_chat_ids,
        }
    }

    fn is_allowed(&self, chat_id: i64) -> bool {
        self.allowed_chat_ids.is_empty() || self.allowed_chat_ids.contains(&chat_id)
    }

    /// Process one batch of updates from the API.
    /// Split out for testability.
    pub fn run_once(
        &self,
        make_handler: &dyn Fn() -> Arc<dyn OrderHandler>,
        handlers: &mut HashMap<i64, Arc<dyn OrderHandler>>,
        offset: &mut i64,
    ) {
        let updates = self.gateway.fetch_updates(*offset);

        for update in updates {
            // Always advance offset, even if we reject the message
            *offset = update.update_id + 1;

            if !self.is_allowed(update.chat_id) {
                eprintln!("[telegram: ignoring unauthorised chat {}]", update.chat_id);
                continue;
            }

            let text = update.text.trim();

            // Handle /reset command
            if text == "/reset" {
                if let Some(handler) = handlers.get(&update.chat_id) {
                    handler.reset_session();
                }
                self.gateway
                    .post_message(update.chat_id, "Sesión reiniciada.");
                continue;
            }

            // Skip other /commands
            if text.starts_with('/') {
                continue;
            }

            // Get or create handler for this chat
            let handler = handlers
                .entry(update.chat_id)
                .or_insert_with(make_handler);

            eprintln!(
                "[telegram chat={} text={:?}]",
                update.chat_id,
                &update.text[..update.text.len().min(50)]
            );
            let response = handler.handle(&update.text);
            let lower = response.to_lowercase();
            if lower.contains("alexa") && lower.contains("spotify") {
                eprintln!("[telegram: alexa+spotify detected, synthesizing voice order]");
                let bytes = synthesize_alexa_spotify(&response);
                if bytes.is_empty() {
                    eprintln!("[telegram: TTS synthesis failed]");
                } else {
                    play_audio_bytes(&bytes);
                }
            }
            self.gateway.post_message(update.chat_id, &response);
        }
    }

    /// Main event loop: fetch updates and process them indefinitely.
    pub fn run(&self, make_handler: impl Fn() -> Arc<dyn OrderHandler>) {
        eprintln!("[telegram bot starting, allowed chats: {:?}]", self.allowed_chat_ids);
        let mut offset: i64 = 0;
        let mut handlers: HashMap<i64, Arc<dyn OrderHandler>> = HashMap::new();

        loop {
            self.run_once(&make_handler, &mut handlers, &mut offset);
        }
    }
}

/// Parse Telegram API response JSON into updates.
/// Uses serde_json::Value for dynamic parsing (avoids complex derive hierarchies).
fn parse_updates(body: &str) -> Vec<TelegramUpdate> {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[telegram json parse error: {e}]");
            return vec![];
        }
    };

    let mut results = Vec::new();
    let updates = match v["result"].as_array() {
        Some(a) => a,
        None => return vec![],
    };

    for update in updates {
        // Support both "message" and "edited_message"
        let msg = update
            .get("message")
            .or_else(|| update.get("edited_message"));
        let msg = match msg {
            Some(m) => m,
            None => continue,
        };

        let update_id = match update["update_id"].as_i64() {
            Some(id) => id,
            None => continue,
        };

        let chat_id = match msg["chat"]["id"].as_i64() {
            Some(id) => id,
            None => continue,
        };

        let text = match msg["text"].as_str() {
            Some(t) => t.to_string(),
            None => continue, // Skip stickers, photos, etc.
        };

        results.push(TelegramUpdate {
            update_id,
            chat_id,
            text,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_updates_returns_empty_on_empty_result() {
        let json = r#"{"ok":true,"result":[]}"#;
        assert!(parse_updates(json).is_empty());
    }

    #[test]
    fn parse_updates_extracts_update_id_chat_id_and_text() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 100,
                "message": {
                    "chat": {"id": 42},
                    "text": "hola"
                }
            }]
        }"#;
        let updates = parse_updates(json);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].update_id, 100);
        assert_eq!(updates[0].chat_id, 42);
        assert_eq!(updates[0].text, "hola");
    }

    #[test]
    fn parse_updates_skips_updates_without_text() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 101,
                "message": {
                    "chat": {"id": 5},
                    "sticker": {}
                }
            }]
        }"#;
        assert!(parse_updates(json).is_empty());
    }

    #[test]
    fn parse_updates_handles_edited_message() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 200,
                "edited_message": {
                    "chat": {"id": 99},
                    "text": "corregido"
                }
            }]
        }"#;
        let updates = parse_updates(json);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].text, "corregido");
    }

    #[test]
    fn parse_updates_returns_empty_on_malformed_json() {
        assert!(parse_updates("not json at all").is_empty());
    }
}
