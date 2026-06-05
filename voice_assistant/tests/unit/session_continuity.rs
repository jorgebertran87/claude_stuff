use cucumber::{given, when, then, World};
use std::path::PathBuf;
use std::sync::Mutex;

use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::claude_handler::{ClaudeBackend, ClaudeCodeHandler, TokenUsage};

// ── Fake backend ────────────────────────────────────────────────────────────

/// We need shared access to both the handler and the backend's call log.
/// Since ClaudeCodeHandler takes ownership of the backend, we use a wrapper
/// that stores the calls in an Arc<Mutex>.
struct SharedBackend {
    session_id_to_return: String,
    calls: std::sync::Arc<Mutex<Vec<Option<String>>>>,
}

impl SharedBackend {
    fn new(session_id: &str) -> (Self, std::sync::Arc<Mutex<Vec<Option<String>>>>) {
        let calls = std::sync::Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                session_id_to_return: session_id.into(),
                calls: calls.clone(),
            },
            calls,
        )
    }
}

impl ClaudeBackend for SharedBackend {
    fn query(&self, _order: &str, session_id: Option<&str>) -> Result<TokenUsage, String> {
        self.calls.lock().unwrap().push(session_id.map(|s| s.to_string()));
        Ok(TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            total_cost_usd: 0.001,
            session_id: Some(self.session_id_to_return.clone()),
            result: "ok".into(),
        })
    }
}

#[derive(World)]
pub struct SessionWorld {
    log_path: PathBuf,
    handler: Option<ClaudeCodeHandler>,
    calls: std::sync::Arc<Mutex<Vec<Option<String>>>>,
    stored_session_id: Option<String>,
}

impl std::fmt::Debug for SessionWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionWorld")
            .field("log_path", &self.log_path)
            .field("stored_session_id", &self.stored_session_id)
            .finish()
    }
}

impl Default for SessionWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("session_test.log");
        std::mem::forget(dir);
        Self {
            log_path,
            handler: None,
            calls: std::sync::Arc::new(Mutex::new(Vec::new())),
            stored_session_id: None,
        }
    }
}

fn setup_handler(world: &mut SessionWorld, session_id: &str) {
    let (backend, calls) = SharedBackend::new(session_id);
    world.calls = calls;
    world.handler = Some(ClaudeCodeHandler::with_injectable(
        std::sync::Arc::new(backend),
        world.log_path.clone(),
    ));
}

// ── Steps ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a ClaudeCodeHandler with a backend that returns session_id "(.+)"$"#)]
fn given_handler_with_session(world: &mut SessionWorld, session_id: String) {
    setup_handler(world, &session_id);
}

#[given("a ClaudeCodeHandler with no prior session")]
fn given_handler_no_session(world: &mut SessionWorld) {
    setup_handler(world, "new-session");
}

#[given("the handler has already handled one order")]
fn given_handled_one(world: &mut SessionWorld) {
    let handler = world.handler.as_ref().unwrap();
    handler.handle("primera orden");
}

#[given("reset_session is called")]
fn given_reset(world: &mut SessionWorld) {
    world.handler.as_ref().unwrap().reset_session();
}

#[when(regex = r#"^the handler handles "(.+)"$"#)]
fn when_handle(world: &mut SessionWorld, order: String) {
    let handler = world.handler.as_ref().unwrap();
    handler.handle(&order);
}

#[when("the handler handles a second order")]
fn when_second_order(world: &mut SessionWorld) {
    let handler = world.handler.as_ref().unwrap();
    handler.handle("segunda orden");
}

#[when("reset_session is called")]
fn when_reset(world: &mut SessionWorld) {
    world.handler.as_ref().unwrap().reset_session();
}

#[when("the handler handles another order")]
fn when_another_order(world: &mut SessionWorld) {
    let handler = world.handler.as_ref().unwrap();
    handler.handle("otra orden");
}

#[then(regex = r#"^the stored session_id is "(.+)"$"#)]
fn then_stored_session(world: &mut SessionWorld, expected: String) {
    // The handler stores session_id internally. After a handle() call, the
    // next query() call should receive the stored session_id.
    // We verify by making another call and checking what the backend received.
    let calls = world.calls.lock().unwrap();
    // The last call should have set the session_id for the *next* call.
    // We verify indirectly: the backend returned session_id "abc-123" on the
    // first call, so the handler should have stored it.
    // We can verify by doing another call and checking:
    drop(calls);
    let handler = world.handler.as_ref().unwrap();
    handler.handle("verify-call");
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    assert_eq!(last.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the backend receives session_id "(.+)" on the second call$"#)]
fn then_backend_receives(world: &mut SessionWorld, expected: String) {
    let calls = world.calls.lock().unwrap();
    // Second call is index 1
    assert!(calls.len() >= 2, "expected at least 2 calls, got {}", calls.len());
    assert_eq!(
        calls[1].as_deref(),
        Some(expected.as_str()),
        "second call should have session_id"
    );
}

#[then("the backend receives no session_id on that call")]
fn then_no_session(world: &mut SessionWorld) {
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    assert!(last.is_none(), "expected no session_id, got {:?}", last);
}

#[then("the stored session_id is None")]
fn then_session_none(world: &mut SessionWorld) {
    // Verify by making a call — it should not send a session_id
    let handler = world.handler.as_ref().unwrap();
    handler.handle("verify-none");
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    assert!(last.is_none(), "expected None session_id after reset, got {:?}", last);
}

fn main() {
    futures::executor::block_on(
        SessionWorld::run("features/session_continuity.feature"),
    );
}
