use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use crate::domain::ports::OrderHandler;

/// Detect which skill to use based on the order text.
pub fn detect_intent(order: &str) -> &'static str {
    let lower = order.to_lowercase();
    if lower.contains("bus") || lower.contains("autobús") || lower.contains("autobus")
        || lower.contains("parada") || lower.contains("línea") || lower.contains("linea")
    {
        "bus"
    } else if lower.contains("música") || lower.contains("musica") || lower.contains("spotify")
        || lower.contains("canción") || lower.contains("cancion") || lower.contains("playlist")
        || lower.contains("reproduce") || lower.contains("pon ")
    {
        "music"
    } else if lower.contains("tiempo") || lower.contains("lluvia") || lower.contains("llover")
        || lower.contains("temperatura")
        || lower.contains("calor") || lower.contains("frío") || lower.contains("frio")
        || lower.contains("clima") || lower.contains("sol") || lower.contains("nube")
        || lower.contains("weather")
    {
        "weather"
    } else {
        "search"
    }
}

/// Strip YAML frontmatter from a skill `.md` file, returning only the prompt body.
pub fn strip_frontmatter(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim().to_string();
        }
    }
    content.trim().to_string()
}

/// Load a skill file by name. Tries several candidate paths so it works both
/// inside Docker (mounted at /app/.claude/commands/) and in local dev.
fn load_skill(name: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let candidates = [
        format!("/app/.claude/commands/{name}.md"),
        format!("{home}/.claude/commands/{name}.md"),
        format!("../.claude/commands/{name}.md"),
        format!(".claude/commands/{name}.md"),
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            return strip_frontmatter(&content);
        }
    }
    eprintln!("[claude: skill '{name}' not found in any candidate path]");
    String::new()
}

/// Build the system prompt for a given order: base behavioural rules + intent-specific skill.
fn load_prompt(order: &str) -> String {
    let voice_language = std::env::var("VOICE_LANGUAGE").unwrap_or_else(|_| "es".into());
    let lang_rule = format!("Responde en el idioma oficial del país con código '{voice_language}'.");

    let base     = load_skill("claudito");
    let specific = load_skill(detect_intent(order));

    [lang_rule, base, specific]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ── Public data types ─────────────────────────────────────────────────────────

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub total_cost_usd: f64,
    pub session_id: Option<String>,
    pub result: String,
}

// ── Backend trait (injectable for tests) ──────────────────────────────────────

pub trait ClaudeBackend: Send + Sync {
    fn query(&self, order: &str, session_id: Option<&str>) -> Result<TokenUsage, String>;
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub struct ClaudeCodeHandler {
    backend:    Box<dyn ClaudeBackend>,
    log_file:   PathBuf,
    session_id: Mutex<Option<String>>,
}

impl ClaudeCodeHandler {
    pub fn new() -> Self {
        Self {
            backend:    Box::new(ClaudeCliBackend),
            log_file:   PathBuf::from(".orders_tokens"),
            session_id: Mutex::new(None),
        }
    }

    pub fn with_injectable(backend: Box<dyn ClaudeBackend>, log_file: PathBuf) -> Self {
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

    fn reset_session(&self) {
        *self.session_id.lock().unwrap() = None;
        eprintln!("[session reset]");
    }
}

// ── Real backend: calls the `claude` CLI ─────────────────────────────────────

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
    let result     = extract_str(json, "\"result\":")         .unwrap_or_default();
    let cost_str   = extract_str(json, "\"total_cost_usd\":") .unwrap_or_default();
    let session_id = extract_str(json, "\"session_id\":");
    let cost: f64  = cost_str.parse().unwrap_or(0.0);

    Ok(TokenUsage {
        input_tokens:              extract_u64(json, "\"input_tokens\":"),
        output_tokens:             extract_u64(json, "\"output_tokens\":"),
        cache_read_input_tokens:   extract_u64(json, "\"cache_read_input_tokens\":"),
        cache_creation_input_tokens: extract_u64(json, "\"cache_creation_input_tokens\":"),
        total_cost_usd: cost,
        session_id,
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
