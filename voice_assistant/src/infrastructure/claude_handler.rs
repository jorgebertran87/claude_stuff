use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::domain::ports::OrderHandler;
use crate::infrastructure::token_usage::log_token_usage;

// Re-exports for backward compatibility with external tests.
pub use crate::infrastructure::skill_loader::{detect_intent, strip_frontmatter, load_skill, load_prompt};
pub use crate::infrastructure::token_usage::{TokenUsage, parse_result_json, extract_u64, extract_str};

pub trait ClaudeBackend: Send + Sync {
    fn query(&self, order: &str, session_id: Option<&str>) -> Result<TokenUsage, String>;
}

pub struct ClaudeCodeHandler {
    backend:    Arc<dyn ClaudeBackend>,
    log_file:   PathBuf,
    session_id: Mutex<Option<String>>,
}

impl ClaudeCodeHandler {
    pub fn new(backend: Arc<dyn ClaudeBackend>, log_file: PathBuf) -> Self {
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

// ── DeepSeekBackend (orders) ──────────────────────────────────────────────────

pub struct DeepSeekBackend {
    config: deepseek_client::DeepSeekConfig,
}

impl DeepSeekBackend {
    pub fn new() -> Self {
        Self { config: deepseek_client::DeepSeekConfig::from_env() }
    }

    pub fn with_base_url(base_url: String, api_key: String, model: String) -> Self {
        Self { config: deepseek_client::DeepSeekConfig::with_base_url(base_url, api_key, model) }
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

