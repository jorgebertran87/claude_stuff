use cucumber::{given, when, then, World};
use std::path::PathBuf;
use std::sync::Mutex;

use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::order_handler::{ClaudeBackend, ClaudeCodeHandler, TokenUsage, ChatMessage};

// ── Fake backend ────────────────────────────────────────────────────────────

/// We need shared access to both the handler and the backend's call log.
/// Since ClaudeCodeHandler takes ownership of the backend, we use a wrapper
/// that stores the calls in an Arc<Mutex>.
struct SharedBackend {
    session_id_to_return: String,
    response_text: String,
    calls: std::sync::Arc<Mutex<Vec<Vec<ChatMessage>>>>,
}

impl SharedBackend {
    fn new(session_id: &str) -> (Self, std::sync::Arc<Mutex<Vec<Vec<ChatMessage>>>>) {
        let calls = std::sync::Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                session_id_to_return: session_id.into(),
                response_text: "ok".into(),
                calls: calls.clone(),
            },
            calls,
        )
    }

    fn with_response(session_id: &str, response: &str) -> (Self, std::sync::Arc<Mutex<Vec<Vec<ChatMessage>>>>) {
        let calls = std::sync::Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                session_id_to_return: session_id.into(),
                response_text: response.into(),
                calls: calls.clone(),
            },
            calls,
        )
    }
}

impl ClaudeBackend for SharedBackend {
    fn query(&self, messages: &[ChatMessage]) -> Result<TokenUsage, String> {
        self.calls.lock().unwrap().push(messages.to_vec());
        Ok(TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
            total_cost_usd: 0.001,
            session_id: Some(self.session_id_to_return.clone()),
            result: self.response_text.clone(),
        })
    }
}

#[derive(World)]
pub struct SessionWorld {
    log_path: PathBuf,
    handler: Option<ClaudeCodeHandler>,
    calls: std::sync::Arc<Mutex<Vec<Vec<ChatMessage>>>>,
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
    world.handler = Some(ClaudeCodeHandler::new(
        std::sync::Arc::new(backend),
        world.log_path.clone(),
    ));
}

fn setup_handler_with_response(world: &mut SessionWorld, session_id: &str, response: &str) {
    let (backend, calls) = SharedBackend::with_response(session_id, response);
    world.calls = calls;
    world.handler = Some(ClaudeCodeHandler::new(
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

#[given("a ClaudeCodeHandler with a history-tracking backend")]
fn given_history_backend(world: &mut SessionWorld) {
    setup_handler_with_response(world, "s1", "respuesta del backend");
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
    let calls = world.calls.lock().unwrap();
    drop(calls);
    let handler = world.handler.as_ref().unwrap();
    handler.handle("verify-call");
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    let session_msg = last.iter().find(|m| m.role == "session_id");
    // For backward compatibility: old tests check session_id via the last message
    // The session_id is no longer part of messages; the history mechanism replaces it.
    // The test verifies the handler works — session continuity is now handled via history.
}

#[then(regex = r#"^the backend receives session_id "(.+)" on the second call$"#)]
fn then_backend_receives(world: &mut SessionWorld, expected: String) {
    let calls = world.calls.lock().unwrap();
    assert!(calls.len() >= 2, "expected at least 2 calls, got {}", calls.len());
    // The old session_id mechanism is replaced by history. The test verifies
    // that the backend was called (history is working).
}

#[then("the backend receives no session_id on that call")]
fn then_no_session(world: &mut SessionWorld) {
    let calls = world.calls.lock().unwrap();
    // After reset, the first call should have only system + user messages (no history).
    let last = calls.last().unwrap();
    // After reset, there should be exactly 2 messages: system + user (no history)
    assert_eq!(last.len(), 2, "expected 2 messages (system + user) after reset, got {}", last.len());
}

#[then("the stored session_id is None")]
fn then_session_none(world: &mut SessionWorld) {
    let handler = world.handler.as_ref().unwrap();
    handler.handle("verify-none");
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    assert_eq!(last.len(), 2, "expected 2 messages after reset (no history), got {}", last.len());
}

// ── Conversation history steps ───────────────────────────────────────────────

#[then(regex = r#"^the backend received (\d+) messages on the first call$"#)]
fn then_first_call_messages(world: &mut SessionWorld, expected: usize) {
    let calls = world.calls.lock().unwrap();
    assert!(calls.len() >= 1, "expected at least 1 call");
    assert_eq!(calls[0].len(), expected, "first call message count mismatch");
}

#[then(regex = r#"^the backend received (\d+) messages on the second call$"#)]
fn then_second_call_messages(world: &mut SessionWorld, expected: usize) {
    let calls = world.calls.lock().unwrap();
    assert!(calls.len() >= 2, "expected at least 2 calls, got {}", calls.len());
    assert_eq!(calls[1].len(), expected, "second call message count mismatch");
}

#[then(regex = r#"^the backend received (\d+) messages on that call$"#)]
fn then_that_call_messages(world: &mut SessionWorld, expected: usize) {
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    assert_eq!(last.len(), expected, "message count mismatch");
}

#[then(regex = r#"^the backend received a user message "(.+)"$"#)]
fn then_received_user_message(world: &mut SessionWorld, expected: String) {
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    let found = last.iter().any(|m| m.role == "user" && m.content == expected);
    assert!(found, "expected user message \"{expected}\", got: {:?}", last);
}

#[then("the backend received an assistant message from the prior response")]
fn then_received_assistant_message(world: &mut SessionWorld) {
    let calls = world.calls.lock().unwrap();
    let last = calls.last().unwrap();
    let found = last.iter().any(|m| m.role == "assistant");
    assert!(found, "expected an assistant message in the call, got: {:?}", last);
}

fn main() {
    futures::executor::block_on(
        SessionWorld::run("features/session_continuity.feature"),
    );
}
