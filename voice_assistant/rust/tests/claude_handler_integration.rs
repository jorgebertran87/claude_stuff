use cucumber::{given, when, then, World};
use std::path::PathBuf;

use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::claude_handler::{ClaudeCliBackend, ClaudeCodeHandler};

#[derive(World)]
pub struct ClaudeCliWorld {
    handler: Option<ClaudeCodeHandler>,
    _temp_dir: Option<tempfile::TempDir>,
    log_path: PathBuf,
    result: String,
}

impl std::fmt::Debug for ClaudeCliWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCliWorld")
            .field("log_path", &self.log_path)
            .field("result", &self.result)
            .finish()
    }
}

impl Default for ClaudeCliWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("integration_tokens.log");
        Self {
            handler: None,
            _temp_dir: Some(dir),
            log_path,
            result: String::new(),
        }
    }
}

#[given("the claude CLI is available and authenticated")]
fn given_claude_available(world: &mut ClaudeCliWorld) {
    world.handler = Some(ClaudeCodeHandler::with_injectable(
        Box::new(ClaudeCliBackend),
        world.log_path.clone(),
    ));
}

#[given("no token log file exists yet")]
fn given_no_log(world: &mut ClaudeCliWorld) {
    let _ = std::fs::remove_file(&world.log_path);
}

#[when(regex = r#"^ClaudeCodeHandler handles "(.+)"$"#)]
fn when_handle(world: &mut ClaudeCliWorld, order: String) {
    let handler = world.handler.as_ref().unwrap();
    world.result = handler.handle(&order);
}

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

fn main() {
    futures::executor::block_on(
        ClaudeCliWorld::run("features/claude_handler_integration.feature"),
    );
}
