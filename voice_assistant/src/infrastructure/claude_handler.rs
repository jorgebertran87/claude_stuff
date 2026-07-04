use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::domain::ports::OrderHandler;
use crate::infrastructure::token_usage::log_token_usage;

// Re-exports for backward compatibility with external tests.
pub use crate::infrastructure::skill_loader::{detect_intent, strip_frontmatter, load_skill, load_prompt};
pub use crate::infrastructure::token_usage::{TokenUsage, parse_result_json, extract_u64, extract_str};
pub use deepseek_client::ChatMessage;

pub trait ClaudeBackend: Send + Sync {
    fn query(&self, messages: &[ChatMessage]) -> Result<TokenUsage, String>;
}

pub struct ClaudeCodeHandler {
    backend:    Arc<dyn ClaudeBackend>,
    log_file:   PathBuf,
    session_id: Mutex<Option<String>>,
    history:    Mutex<Vec<ChatMessage>>,
}

impl ClaudeCodeHandler {
    pub fn new(backend: Arc<dyn ClaudeBackend>, log_file: PathBuf) -> Self {
        Self {
            backend,
            log_file,
            session_id: Mutex::new(None),
            history: Mutex::new(Vec::new()),
        }
    }
}

impl OrderHandler for ClaudeCodeHandler {
    fn handle(&self, order: &str) -> String {
        // Build the system prompt for this order (intent-based skill loading).
        let system_prompt = load_prompt(order);

        // Assemble the full message list: system prompt + history + current user order.
        let mut messages: Vec<ChatMessage> = Vec::new();
        messages.push(ChatMessage {
            role: "system".into(),
            content: system_prompt,
        });
        {
            let history = self.history.lock().unwrap();
            messages.extend(history.clone());
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: order.to_string(),
        });

        let session_id = self.session_id.lock().unwrap().clone();
        match self.backend.query(&messages) {
            Err(e) => {
                eprintln!("[claude handler error: {e}]");
                "No tienes tokens disponibles. Por favor, revisa tu configuración.".into()
            }
            Ok(usage) => {
                *self.session_id.lock().unwrap() = usage.session_id.clone();

                // Store this turn in conversation history.
                {
                    let mut history = self.history.lock().unwrap();
                    history.push(ChatMessage {
                        role: "user".into(),
                        content: order.to_string(),
                    });
                    history.push(ChatMessage {
                        role: "assistant".into(),
                        content: usage.result.clone(),
                    });
                }

                log_token_usage(order, &usage, self.log_file.to_str().unwrap_or(".orders_tokens"));
                usage.result
            }
        }
    }

    fn reset_session(&self) {
        *self.session_id.lock().unwrap() = None;
        self.history.lock().unwrap().clear();
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
    fn query(&self, messages: &[ChatMessage]) -> Result<TokenUsage, String> {
        let resp = deepseek_client::chat(
            &self.config.base_url,
            &self.config.api_key,
            &self.config.model,
            messages,
            self.config.reasoning_effort.as_deref(),
        )?;

        let preview: String = resp.content.chars().take(200).collect();
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
