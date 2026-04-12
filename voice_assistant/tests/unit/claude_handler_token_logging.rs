use cucumber::{given, when, then, World};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::claude_handler::{ClaudeBackend, ClaudeCodeHandler, TokenUsage};

// ── Fake backend ─────────────────────────────────────────────────────────────

struct FakeBackend {
    result: String,
    input_tokens: u64,
    output_tokens: u64,
    cache_read: u64,
    cache_creation: u64,
    cost: f64,
}

impl FakeBackend {
    fn default_mock() -> Self {
        Self {
            result: "ok".into(),
            input_tokens: 10,
            output_tokens: 20,
            cache_read: 30,
            cache_creation: 40,
            cost: 0.001,
        }
    }
}

impl ClaudeBackend for FakeBackend {
    fn query(&self, _order: &str, _session_id: Option<&str>) -> Result<TokenUsage, String> {
        Ok(TokenUsage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_read_input_tokens: self.cache_read,
            cache_creation_input_tokens: self.cache_creation,
            total_cost_usd: self.cost,
            session_id: Some("test-session".into()),
            result: self.result.clone(),
        })
    }
}

struct SessionTrackingBackend {
    calls: Arc<Mutex<Vec<Option<String>>>>,
}

impl ClaudeBackend for SessionTrackingBackend {
    fn query(&self, _order: &str, session_id: Option<&str>) -> Result<TokenUsage, String> {
        self.calls.lock().unwrap().push(session_id.map(str::to_string));
        Ok(TokenUsage {
            input_tokens: 1, output_tokens: 1,
            cache_read_input_tokens: 0, cache_creation_input_tokens: 0,
            total_cost_usd: 0.0,
            session_id: Some("tracked-session".to_string()),
            result: "ok".to_string(),
        })
    }
}

// ── World ────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct TokenLogWorld {
    log_path: PathBuf,
    handler: Option<ClaudeCodeHandler>,
    return_value: String,
    session_calls: Option<Arc<Mutex<Vec<Option<String>>>>>,
}

impl std::fmt::Debug for TokenLogWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenLogWorld")
            .field("log_path", &self.log_path)
            .field("return_value", &self.return_value)
            .finish()
    }
}

impl Default for TokenLogWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test_tokens.log");
        // We leak the dir so it lives for the scenario duration
        std::mem::forget(dir);
        Self {
            log_path,
            handler: None,
            return_value: String::new(),
            session_calls: None,
        }
    }
}

fn make_handler(world: &mut TokenLogWorld, backend: FakeBackend) {
    world.handler = Some(ClaudeCodeHandler::with_injectable(
        Arc::new(backend),
        world.log_path.clone(),
    ));
}

// ── Steps ────────────────────────────────────────────────────────────────────

#[given("a ClaudeCodeHandler with a mocked query that returns a result message")]
fn given_mocked_handler(world: &mut TokenLogWorld) {
    make_handler(world, FakeBackend::default_mock());
}

#[given(regex = r"^a ClaudeCodeHandler with a mocked query returning input_tokens=(\d+), output_tokens=(\d+), cache_read=(\d+), cache_creation=(\d+), cost=(.+)$")]
fn given_specific_tokens(
    world: &mut TokenLogWorld,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_creation: u64,
    cost: f64,
) {
    make_handler(world, FakeBackend {
        result: "respuesta".into(),
        input_tokens: input,
        output_tokens: output,
        cache_read,
        cache_creation,
        cost,
    });
}

#[given("a ClaudeCodeHandler with a mocked query")]
fn given_simple_mock(world: &mut TokenLogWorld) {
    make_handler(world, FakeBackend::default_mock());
}

#[given(regex = r#"^a ClaudeCodeHandler with a mocked query that returns result "(.+)"$"#)]
fn given_mock_with_result(world: &mut TokenLogWorld, result: String) {
    make_handler(world, FakeBackend {
        result,
        ..FakeBackend::default_mock()
    });
}

#[given("a session-tracking backend")]
fn given_session_tracking(world: &mut TokenLogWorld) {
    let calls = Arc::new(Mutex::new(Vec::<Option<String>>::new()));
    world.session_calls = Some(calls.clone());
    world.handler = Some(ClaudeCodeHandler::with_injectable(
        Arc::new(SessionTrackingBackend { calls }),
        world.log_path.clone(),
    ));
}

#[when(regex = r#"^the handler handles "(.+)"$"#)]
fn when_handle(world: &mut TokenLogWorld, order: String) {
    let handler = world.handler.as_ref().unwrap();
    world.return_value = handler.handle(&order);
}

#[when("reset_session is called")]
fn when_reset_session(world: &mut TokenLogWorld) {
    world.handler.as_ref().unwrap().reset_session();
}

#[then("the token log file exists")]
fn then_log_exists(world: &mut TokenLogWorld) {
    assert!(world.log_path.exists(), "log file should exist at {:?}", world.log_path);
}

#[then(regex = r#"^the log line contains "(.+)"$"#)]
fn then_log_contains(world: &mut TokenLogWorld, needle: String) {
    let content = std::fs::read_to_string(&world.log_path).unwrap();
    assert!(
        content.contains(&needle),
        "log should contain \"{needle}\", got:\n{content}"
    );
}

#[then(regex = r"^the log file has exactly (\d+) lines$")]
fn then_line_count(world: &mut TokenLogWorld, expected: usize) {
    let content = std::fs::read_to_string(&world.log_path).unwrap();
    let count = content.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(count, expected, "expected {expected} lines, got {count}");
}

#[then(regex = r#"^line (\d+) contains "(.+)"$"#)]
fn then_specific_line_contains(world: &mut TokenLogWorld, line_num: usize, needle: String) {
    let content = std::fs::read_to_string(&world.log_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert!(
        lines.len() >= line_num,
        "expected at least {line_num} lines, got {}",
        lines.len()
    );
    assert!(
        lines[line_num - 1].contains(&needle),
        "line {line_num} should contain \"{needle}\", got: {}",
        lines[line_num - 1]
    );
}

#[then(regex = r#"^the return value is "(.+)"$"#)]
fn then_return_value(world: &mut TokenLogWorld, expected: String) {
    assert_eq!(world.return_value, expected);
}

#[then("the second call had no session id")]
fn then_second_call_no_session(world: &mut TokenLogWorld) {
    let calls = world.session_calls.as_ref().unwrap().lock().unwrap();
    let second = calls.get(1);
    assert_eq!(
        second, Some(&None),
        "second call should have had no session id, but got: {:?}", second,
    );
}

fn main() {
    futures::executor::block_on(
        TokenLogWorld::run("features/claude_handler_token_logging.feature"),
    );
}
