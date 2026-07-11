use std::io::Read;
use std::time::Duration;

use serde_json::Value;
use shaku::Component;

#[derive(Clone)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub chat_id: i64,
    /// Message text or caption (empty string when only a photo was sent).
    pub text: String,
    /// file_id of the largest available photo, if the message contains an image.
    pub photo_file_id: Option<String>,
}

pub trait TelegramGateway: Send + Sync {
    fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate>;
    fn post_message(&self, chat_id: i64, text: &str);
    fn send_voice(&self, chat_id: i64, data: &[u8]);
    fn download_file(&self, file_id: &str) -> Option<Vec<u8>>;
}

#[derive(Component)]
#[shaku(interface = TelegramGateway)]
pub struct UreqGateway {
    token: String,
}

impl UreqGateway {
    pub fn new(token: String) -> Self {
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

/// Returns true when the `getUpdates` error should be logged as a real error.
/// Long-poll timeouts (Telegram's `timeout=30` closing the connection after an
/// idle window) are expected and should be suppressed to avoid noise.
fn should_log_get_updates_error(error_msg: &str) -> bool {
    !error_msg.contains("timed out reading response")
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
                let err_msg = e.to_string();
                if should_log_get_updates_error(&err_msg) {
                    eprintln!("[telegram get_updates error: {err_msg}]");
                }
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

    fn download_file(&self, file_id: &str) -> Option<Vec<u8>> {
        let url = format!("{}/getFile?file_id={}", self.base_url(), file_id);
        let resp = ureq::get(&url)
            .timeout(Duration::from_secs(30))
            .call()
            .ok()?;
        let body = resp.into_string().ok()?;
        let v: Value = serde_json::from_str(&body).ok()?;
        let file_path = v["result"]["file_path"].as_str()?;
        let file_url = format!("https://api.telegram.org/file/bot{}/{}", self.token, file_path);
        let resp = ureq::get(&file_url)
            .timeout(Duration::from_secs(60))
            .call()
            .ok()?;
        let mut bytes = Vec::new();
        resp.into_reader().read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    fn send_voice(&self, chat_id: i64, data: &[u8]) {
        let url = format!("{}/sendVoice", self.base_url());
        let boundary = "TelegramVoiceBoundary";
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"chat_id\"\r\n\r\n");
        body.extend_from_slice(chat_id.to_string().as_bytes());
        body.extend_from_slice(b"\r\n");
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

pub fn parse_updates(body: &str) -> Vec<TelegramUpdate> {
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

        let doc = &msg["document"];
        let doc_image_file_id = doc["mime_type"].as_str()
            .filter(|mime| mime.starts_with("image/"))
            .and_then(|_| doc["file_id"].as_str().map(|s| s.to_string()));

        let photo_file_id = doc_image_file_id.or_else(|| {
            msg["photo"].as_array().and_then(|photos| {
                photos.last().and_then(|p| p["file_id"].as_str().map(|s| s.to_string()))
            })
        });

        let text = match msg["text"].as_str() {
            Some(t) => t.to_string(),
            None if photo_file_id.is_some() => {
                msg["caption"].as_str().unwrap_or("").to_string()
            }
            None => continue,
        };

        results.push(TelegramUpdate { update_id, chat_id, text, photo_file_id });
    }

    results
}