use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use shaku::Component;

use crate::domain::ports::{ImageAnalyzer, OrderHandler};
use crate::infrastructure::token_usage::log_token_usage;

// Re-exports for backward compatibility with external tests.
pub use crate::infrastructure::skill_loader::{detect_intent, strip_frontmatter, load_skill, load_prompt};
pub use crate::infrastructure::token_usage::{TokenUsage, parse_result_json, extract_u64, extract_str};

pub trait ClaudeBackend: Send + Sync {
    fn query(&self, order: &str, session_id: Option<&str>) -> Result<TokenUsage, String>;
}

#[derive(Component)]
#[shaku(interface = OrderHandler)]
pub struct ClaudeCodeHandler {
    #[shaku(inject)]
    backend:    Arc<dyn ClaudeBackend>,
    log_file:   PathBuf,
    #[shaku(default)]
    session_id: Mutex<Option<String>>,
}

impl ClaudeCodeHandler {
    pub fn new() -> Self {
        Self {
            backend:    Arc::new(ClaudeCliBackend),
            log_file:   PathBuf::from(".orders_tokens"),
            session_id: Mutex::new(None),
        }
    }

    pub fn with_injectable(backend: Arc<dyn ClaudeBackend>, log_file: PathBuf) -> Self {
        Self { backend, log_file, session_id: Mutex::new(None) }
    }
}

impl OrderHandler for ClaudeCodeHandler {
    fn handle(&self, order: &str) -> String {
        let session_id = self.session_id.lock().unwrap().clone();
        match self.backend.query(order, session_id.as_deref()) {
            Err(e) => {
                eprintln!("[claude handler error: {e}]");
                "No tienes tokens disponibles. Por favor, revisa tu configuración.".into()
            }
            Ok(usage) => {
                *self.session_id.lock().unwrap() = usage.session_id.clone();
                log_token_usage(order, &usage, self.log_file.to_str().unwrap_or(".orders_tokens"));
                usage.result
            }
        }
    }

    fn reset_session(&self) {
        *self.session_id.lock().unwrap() = None;
        eprintln!("[session reset]");
    }
}

// ── ClaudeCliBackend (kept for skills / image analysis) ───────────────────────

#[derive(Component)]
#[shaku(interface = ClaudeBackend)]
pub struct ClaudeCliBackend;

impl ClaudeBackend for ClaudeCliBackend {
    fn query(&self, order: &str, session_id: Option<&str>) -> Result<TokenUsage, String> {
        let prompt = load_prompt(order);
        let mut cmd = Command::new("claude");
        cmd.args(["--print", "--output-format", "json", "--model", "claude-haiku-4-5",
                  "--allowedTools", "Bash,WebSearch"]);
        if let Some(id) = session_id {
            eprintln!("[resuming session: {id}]");
            cmd.args(["--resume", id, "--system-prompt", &prompt]);
        } else {
            cmd.args(["--system-prompt", &prompt]);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(order.as_bytes());
        }

        let output = child.wait_with_output().map_err(|e| e.to_string())?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).into_owned();
            eprintln!("[claude exited with error: {err}]");
            return Err(err);
        }

        let json = String::from_utf8_lossy(&output.stdout);
        let preview_end = json.char_indices().nth(200).map(|(i, _)| i).unwrap_or(json.len());
        eprintln!("[claude raw json: {}]", &json[..preview_end]);
        parse_result_json(&json)
    }
}

// ── DeepSeekBackend (orders) ──────────────────────────────────────────────────

pub struct DeepSeekBackend {
    config: deepseek_client::DeepSeekConfig,
}

impl DeepSeekBackend {
    pub fn new() -> Self {
        Self { config: deepseek_client::DeepSeekConfig::from_env() }
    }

    pub fn with_base_url(base_url: String, api_key: String, model: String) -> Self {
        Self {
            config: deepseek_client::DeepSeekConfig {
                base_url,
                api_key,
                model,
                reasoning_effort: None,
            },
        }
    }
}

impl ClaudeBackend for DeepSeekBackend {
    fn query(&self, order: &str, _session_id: Option<&str>) -> Result<TokenUsage, String> {
        let prompt = load_prompt(order);

        let resp = deepseek_client::chat(
            &self.config.base_url,
            &self.config.api_key,
            &self.config.model,
            &prompt,
            order,
            self.config.reasoning_effort.as_deref(),
        )?;

        let preview = if resp.content.len() > 200 {
            &resp.content[..200]
        } else {
            &resp.content
        };
        eprintln!("[deepseek response: {preview}]");

        Ok(TokenUsage {
            input_tokens: resp.input_tokens,
            output_tokens: resp.output_tokens,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            total_cost_usd: 0.0,
            session_id: None,
            result: resp.content,
        })
    }
}

#[derive(Component)]
#[shaku(interface = ImageAnalyzer)]
pub struct ClaudeImageAnalyzer;

impl ImageAnalyzer for ClaudeImageAnalyzer {
    fn analyze(&self, bytes: &[u8], caption: &str, model: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let debug_dir = std::env::var("TELEGRAM_IMAGE_DEBUG_DIR").ok();
        let dir = debug_dir.as_deref().unwrap_or("/tmp");
        let tmp_path = format!("{dir}/telegram_image_{nanos}.jpg");

        if let Err(e) = std::fs::write(&tmp_path, bytes) {
            eprintln!("[analyze_image: failed to write temp file: {e}]");
            return "Error al procesar la imagen.".to_string();
        }
        eprintln!("[analyze_image: saved image to {tmp_path}]");

        let prompt = if caption.is_empty() { "Describe esta imagen." } else { caption };
        let full_prompt = format!("Read the image at {tmp_path} and then answer: {prompt}");

        let mut child = match Command::new("claude")
            .args(["--print", "--output-format", "json",
                   "--model", model,
                   "--allowedTools", "Read"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[analyze_image: failed to spawn claude: {e}]");
                return "Error al analizar la imagen.".to_string();
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(full_prompt.as_bytes());
        }

        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[analyze_image: wait_with_output error: {e}]");
                return "Error al analizar la imagen.".to_string();
            }
        };

        if debug_dir.is_none() {
            let _ = std::fs::remove_file(&tmp_path);
        }

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            eprintln!("[analyze_image: claude exited with error: {err}]");
            return "Error al analizar la imagen.".to_string();
        }

        let json = String::from_utf8_lossy(&output.stdout);
        match parse_result_json(&json) {
            Ok(usage) => {
                let order_preview = if caption.len() > 80 { &caption[..80] } else { caption };
                log_token_usage(&format!("[image] {order_preview}"), &usage, ".orders_tokens");
                usage.result
            }
            Err(_) => "No se pudo analizar la imagen.".to_string(),
        }
    }
}
