//! Tests for ClaudeCodeHandler token logging.
//! Detroit School: hand-rolled fakes, no mock library.

use std::path::PathBuf;

use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::claude_handler::{
    ClaudeBackend, ClaudeCodeHandler, TokenUsage, detect_intent, strip_frontmatter,
};

// ── Fake backend ──────────────────────────────────────────────────────────────

struct FakeBackend {
    result:               String,
    input_tokens:         u64,
    output_tokens:        u64,
    cache_read:           u64,
    cache_creation:       u64,
    total_cost_usd:       f64,
}

impl FakeBackend {
    fn with_result(result: &str) -> Self {
        Self {
            result:         result.into(),
            input_tokens:   18,
            output_tokens:  735,
            cache_read:     38335,
            cache_creation: 2610,
            total_cost_usd: 0.029965,
        }
    }
}

impl ClaudeBackend for FakeBackend {
    fn query(&self, _order: &str, _session_id: Option<&str>) -> Result<TokenUsage, String> {
        Ok(TokenUsage {
            input_tokens:               self.input_tokens,
            output_tokens:              self.output_tokens,
            cache_read_input_tokens:    self.cache_read,
            cache_creation_input_tokens: self.cache_creation,
            total_cost_usd:             self.total_cost_usd,
            session_id:                 None,
            result:                     self.result.clone(),
        })
    }
}

fn handler_with_log(log_file: &PathBuf) -> ClaudeCodeHandler {
    ClaudeCodeHandler::with_injectable(
        Box::new(FakeBackend::with_result("respuesta")),
        log_file.clone(),
    )
}

// ── Scenario 1: log file is created after handle ──────────────────────────────

#[test]
fn token_log_file_is_created_after_handle() {
    let dir = tempfile::tempdir().unwrap();
    let log_file = dir.path().join(".orders_tokens");

    handler_with_log(&log_file).handle("pon música");

    assert!(log_file.exists());
}

// ── Scenario 2: log contains order and all token fields ───────────────────────

#[test]
fn token_log_contains_order_and_all_token_fields() {
    let dir = tempfile::tempdir().unwrap();
    let log_file = dir.path().join(".orders_tokens");

    ClaudeCodeHandler::with_injectable(
        Box::new(FakeBackend {
            result:         "ok".into(),
            input_tokens:   18,
            output_tokens:  735,
            cache_read:     38335,
            cache_creation: 2610,
            total_cost_usd: 0.029965,
        }),
        log_file.clone(),
    )
    .handle("mañana lloverá");

    let line = std::fs::read_to_string(&log_file).unwrap();
    assert!(line.contains("mañana lloverá"),  "missing order");
    assert!(line.contains("input: 18"),       "missing input_tokens");
    assert!(line.contains("output: 735"),     "missing output_tokens");
    assert!(line.contains("cache_read: 38335"),    "missing cache_read");
    assert!(line.contains("cache_creation: 2610"), "missing cache_creation");
    assert!(line.contains("total: 41698"),    "missing total");
    assert!(line.contains("0.029965"),        "missing cost");
}

// ── Scenario 3: one line appended per call ────────────────────────────────────

#[test]
fn token_log_appends_one_line_per_call() {
    let dir = tempfile::tempdir().unwrap();
    let log_file = dir.path().join(".orders_tokens");

    for order in &["primera orden", "segunda orden"] {
        ClaudeCodeHandler::with_injectable(
            Box::new(FakeBackend::with_result(order)),
            log_file.clone(),
        )
        .handle(order);
    }

    let content = std::fs::read_to_string(&log_file).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("primera orden"));
    assert!(lines[1].contains("segunda orden"));
}

// ── detect_intent ─────────────────────────────────────────────────────────────

#[test]
fn detect_intent_bus_keywords() {
    assert_eq!(detect_intent("cuándo pasa el bus"), "bus");
    assert_eq!(detect_intent("línea 19 hacia el centro"), "bus");
    assert_eq!(detect_intent("parada de autobús"), "bus");
}

#[test]
fn detect_intent_music_keywords() {
    assert_eq!(detect_intent("pon jazz en spotify"), "music");
    assert_eq!(detect_intent("busca la playlist top 50"), "music");
    assert_eq!(detect_intent("reproduce algo de rock"), "music");
    assert_eq!(detect_intent("quiero escuchar música"), "music");
}

#[test]
fn detect_intent_weather_keywords() {
    assert_eq!(detect_intent("qué tiempo hace hoy"), "weather");
    assert_eq!(detect_intent("va a llover mañana"), "weather");
    assert_eq!(detect_intent("cuántos grados hay"), "weather");
    assert_eq!(detect_intent("hace mucho frío"), "weather");
}

#[test]
fn detect_intent_search_is_the_fallback() {
    assert_eq!(detect_intent("cuánto es 347 por 19"), "search");
    assert_eq!(detect_intent("qué es la arquitectura hexagonal"), "search");
    assert_eq!(detect_intent("noticias de hoy"), "search");
}

// ── strip_frontmatter ─────────────────────────────────────────────────────────

#[test]
fn strip_frontmatter_removes_yaml_block() {
    let input = "---\ndescription: test\nallowed-tools: Bash\n---\n\nPrompt body here.";
    assert_eq!(strip_frontmatter(input), "Prompt body here.");
}

#[test]
fn strip_frontmatter_returns_content_unchanged_when_no_frontmatter() {
    let input = "Plain prompt with no frontmatter.";
    assert_eq!(strip_frontmatter(input), input);
}

#[test]
fn strip_frontmatter_trims_whitespace() {
    let input = "---\nkey: val\n---\n\n\n  Body text.  \n";
    assert_eq!(strip_frontmatter(input), "Body text.");
}

// ── Scenario 4: handle returns the result from the message ────────────────────

#[test]
fn handle_returns_result_from_message() {
    let dir = tempfile::tempdir().unwrap();
    let log_file = dir.path().join(".orders_tokens");

    let result = ClaudeCodeHandler::with_injectable(
        Box::new(FakeBackend::with_result("respuesta esperada")),
        log_file,
    )
    .handle("una orden");

    assert_eq!(result, "respuesta esperada");
}
