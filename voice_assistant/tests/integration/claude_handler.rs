use cucumber::{given, when, then, World};
use std::path::PathBuf;

use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::claude_handler::{
    ClaudeCliBackend, ClaudeCodeHandler,
    detect_intent, strip_frontmatter, extract_u64, extract_str,
    load_skill, load_prompt,
};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct ClaudeCliWorld {
    handler:          Option<ClaudeCodeHandler>,
    _temp_dir:        Option<tempfile::TempDir>,
    log_path:         PathBuf,
    result:           String,
    detected_intent:  String,
    stripped_text:    String,
    u64_result:       u64,
    string_result:    Option<String>,
    skill_content:    String,
    prompt_content:   String,
}

impl std::fmt::Debug for ClaudeCliWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCliWorld")
            .field("log_path", &self.log_path)
            .field("result", &self.result)
            .field("detected_intent", &self.detected_intent)
            .finish()
    }
}

impl Default for ClaudeCliWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("integration_tokens.log");
        Self {
            handler:         None,
            _temp_dir:       Some(dir),
            log_path,
            result:          String::new(),
            detected_intent: String::new(),
            stripped_text:   String::new(),
            u64_result:      0,
            string_result:   None,
            skill_content:   String::new(),
            prompt_content:  String::new(),
        }
    }
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given("the claude CLI is available and authenticated")]
fn given_claude_available(world: &mut ClaudeCliWorld) {
    world.handler = Some(ClaudeCodeHandler::with_injectable(
        std::sync::Arc::new(ClaudeCliBackend),
        world.log_path.clone(),
    ));
}

#[given("no token log file exists yet")]
fn given_no_log(world: &mut ClaudeCliWorld) {
    let _ = std::fs::remove_file(&world.log_path);
}

#[given(regex = r#"^a skill file "(.+)" with content "(.+)"$"#)]
fn given_skill_file(_world: &mut ClaudeCliWorld, name: String, content: String) {
    let dir = std::path::Path::new("/app/.claude/commands");
    std::fs::create_dir_all(dir).ok();
    std::fs::write(dir.join(format!("{name}.md")), &content).expect("write test skill file");
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when(regex = r#"^ClaudeCodeHandler handles "(.+)"$"#)]
fn when_handle(world: &mut ClaudeCliWorld, order: String) {
    let handler = world.handler.as_ref().unwrap();
    world.result = handler.handle(&order);
}

#[when(regex = r#"^detect_intent is called with "(.+)"$"#)]
fn when_detect_intent(world: &mut ClaudeCliWorld, order: String) {
    world.detected_intent = detect_intent(&order).to_string();
}

#[when(regex = r#"^strip_frontmatter is called with "(.+)"$"#)]
fn when_strip_frontmatter(world: &mut ClaudeCliWorld, raw: String) {
    let content = raw.replace("\\n", "\n");
    world.stripped_text = strip_frontmatter(&content);
}

#[when(regex = r#"^extract_u64 parses key "(.+)" with value (\d+) from json$"#)]
fn when_extract_u64(world: &mut ClaudeCliWorld, key: String, value: u64) {
    let json = format!("{{\"{key}\": {value}, \"other\": 999}}");
    world.u64_result = extract_u64(&json, &format!("\"{key}\":"));
}

#[when(regex = r#"^extract_str parses key "(.+)" with unquoted value "(.+)" from json$"#)]
fn when_extract_str(world: &mut ClaudeCliWorld, key: String, value: String) {
    let json = format!("{{\"{key}\": {value}, \"other\": \"stuff\"}}");
    world.string_result = extract_str(&json, &format!("\"{key}\":"));
}

#[when(regex = r#"^load_skill is called for "(.+)"$"#)]
fn when_load_skill(world: &mut ClaudeCliWorld, name: String) {
    world.skill_content = load_skill(&name);
}

#[when(regex = r#"^load_prompt is called for "(.+)"$"#)]
fn when_load_prompt(world: &mut ClaudeCliWorld, order: String) {
    world.prompt_content = load_prompt(&order);
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then("the returned string is non-empty")]
fn then_non_empty(world: &mut ClaudeCliWorld) {
    assert!(!world.result.is_empty(), "result should not be empty");
}

#[then("the token log file exists on disk")]
fn then_log_exists(world: &mut ClaudeCliWorld) {
    assert!(
        world.log_path.exists(),
        "token log file should exist at {:?}",
        world.log_path
    );
}

#[then(regex = r#"^the token log contains (?:the text )?"(.+)"$"#)]
fn then_log_contains(world: &mut ClaudeCliWorld, needle: String) {
    let content = std::fs::read_to_string(&world.log_path).unwrap_or_default();
    assert!(content.contains(&needle), "log should contain \"{needle}\"");
}

#[then(regex = r"^the token log file has exactly (\d+) lines$")]
fn then_line_count(world: &mut ClaudeCliWorld, expected: usize) {
    let content = std::fs::read_to_string(&world.log_path).unwrap_or_default();
    let count = content.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(count, expected, "expected {expected} log lines, got {count}");
}

#[then(regex = r#"^the detected intent is "(.+)"$"#)]
fn then_detected_intent(world: &mut ClaudeCliWorld, expected: String) {
    assert_eq!(world.detected_intent, expected,
        "detect_intent returned '{}', expected '{expected}'", world.detected_intent);
}

#[then(regex = r#"^the stripped text is "(.+)"$"#)]
fn then_stripped_text(world: &mut ClaudeCliWorld, expected: String) {
    assert_eq!(world.stripped_text, expected,
        "stripped text was '{}', expected '{expected}'", world.stripped_text);
}

#[then(regex = r"^the u64 result is (\d+)$")]
fn then_u64_result(world: &mut ClaudeCliWorld, expected: u64) {
    assert_eq!(world.u64_result, expected,
        "extract_u64 returned {}, expected {expected}", world.u64_result);
}

#[then(regex = r#"^the string result is "(.+)"$"#)]
fn then_string_result(world: &mut ClaudeCliWorld, expected: String) {
    assert_eq!(world.string_result, Some(expected.clone()),
        "extract_str returned {:?}, expected Some(\"{expected}\")", world.string_result);
}

#[then(regex = r#"^the skill content equals "(.+)"$"#)]
fn then_skill_content(world: &mut ClaudeCliWorld, expected: String) {
    assert_eq!(world.skill_content, expected,
        "load_skill returned '{}', expected '{expected}'", world.skill_content);
}

#[then(regex = r#"^the prompt contains "(.+)"$"#)]
fn then_prompt_contains(world: &mut ClaudeCliWorld, needle: String) {
    assert!(world.prompt_content.contains(&needle),
        "prompt should contain '{needle}', but was: {}", world.prompt_content);
}

fn main() {
    futures::executor::block_on(
        ClaudeCliWorld::run("features/claude_handler_integration.feature"),
    );
}
