use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::domain::ports::OrderHandler;

// `prompt` is gitignored; build.rs copies prompt.example → prompt when absent.
const PROMPT_TEMPLATE: &str = include_str!("prompt");

fn load_prompt() -> String {
    let voice_language   = std::env::var("VOICE_LANGUAGE").unwrap_or_else(|_| "es".into());
    let default_user_city = std::env::var("DEFAULT_USER_CITY").unwrap_or_default();
    PROMPT_TEMPLATE
        .replace("{voice_language}", &voice_language)
        .replace("{default_user_city}", &default_user_city)
}

// ── Public data types ─────────────────────────────────────────────────────────

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub total_cost_usd: f64,
    pub result: String,
}

// ── Backend trait (injectable for tests) ──────────────────────────────────────

pub trait ClaudeBackend: Send + Sync {
    fn query(&self, order: &str) -> Result<TokenUsage, String>;
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub struct ClaudeCodeHandler {
    backend:  Box<dyn ClaudeBackend>,
    log_file: PathBuf,
}

impl ClaudeCodeHandler {
    pub fn new() -> Self {
        Self {
            backend:  Box::new(ClaudeCliBackend),
            log_file: PathBuf::from(".orders_tokens"),
        }
    }

    pub fn with_injectable(backend: Box<dyn ClaudeBackend>, log_file: PathBuf) -> Self {
        Self { backend, log_file }
    }
}

impl OrderHandler for ClaudeCodeHandler {
    fn handle(&self, order: &str) -> String {
        match self.backend.query(order) {
            Err(e) => {
                eprintln!("[claude handler error: {e}]");
                "No tienes tokens disponibles. Por favor, revisa tu configuración.".into()
            }
            Ok(usage) => {
                let total = usage.input_tokens
                    + usage.output_tokens
                    + usage.cache_read_input_tokens
                    + usage.cache_creation_input_tokens;
                let log_line = format!(
                    "Claude order: {} | Tokens used — input: {}, output: {}, \
                     cache_read: {}, cache_creation: {}, total: {} | cost: ${:.6} USD",
                    order,
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.cache_read_input_tokens,
                    usage.cache_creation_input_tokens,
                    total,
                    usage.total_cost_usd,
                );
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.log_file)
                {
                    let _ = writeln!(file, "{}", log_line);
                }
                usage.result
            }
        }
    }
}

// ── Real backend: calls the `claude` CLI ─────────────────────────────────────

struct ClaudeCliBackend;

impl ClaudeBackend for ClaudeCliBackend {
    fn query(&self, order: &str) -> Result<TokenUsage, String> {
        let prompt = load_prompt();
        let mut child = Command::new("claude")
            .args([
                "--print",
                "--output-format", "json",
                "--model", "claude-haiku-4-5",
                "--system-prompt", &prompt,
                "--allowedTools", "Bash,WebSearch",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        // Pass the order via stdin to avoid --allowedTools consuming it as a tool name
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
        eprintln!("[claude raw json: {}]", &json[..json.len().min(200)]);
        parse_result_json(&json)
    }
}

fn parse_result_json(json: &str) -> Result<TokenUsage, String> {
    let result   = extract_str(json, "\"result\":")    .unwrap_or_default();
    let cost_str = extract_str(json, "\"total_cost_usd\":").unwrap_or_default();
    let cost: f64 = cost_str.parse().unwrap_or(0.0);

    Ok(TokenUsage {
        input_tokens:              extract_u64(json, "\"input_tokens\":"),
        output_tokens:             extract_u64(json, "\"output_tokens\":"),
        cache_read_input_tokens:   extract_u64(json, "\"cache_read_input_tokens\":"),
        cache_creation_input_tokens: extract_u64(json, "\"cache_creation_input_tokens\":"),
        total_cost_usd: cost,
        result,
    })
}

fn extract_u64(json: &str, key: &str) -> u64 {
    json.find(key)
        .and_then(|pos| {
            let rest = json[pos + key.len()..].trim_start();
            rest.split(|c: char| !c.is_ascii_digit()).next()
        })
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn extract_str(json: &str, key: &str) -> Option<String> {
    let pos = json.find(key)?;
    let rest = json[pos + key.len()..].trim_start();
    if rest.starts_with('"') {
        let inner = &rest[1..];
        // Walk char by char so we skip over escaped quotes \"
        let mut result = String::new();
        let mut chars = inner.chars();
        loop {
            match chars.next()? {
                '\\' => match chars.next()? {
                    '"'  => result.push('"'),
                    'n'  => result.push('\n'),
                    't'  => result.push('\t'),
                    '\\' => result.push('\\'),
                    c    => { result.push('\\'); result.push(c); }
                },
                '"' => return Some(result),
                c   => result.push(c),
            }
        }
    } else {
        let end = rest.find(|c: char| c == ',' || c == '}' || c == '\n')?;
        Some(rest[..end].trim().to_string())
    }
}
