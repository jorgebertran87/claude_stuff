use cucumber::{given, then, when, World};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::claude_handler::{ChatMessage, ClaudeBackend, ClaudeCodeHandler, TokenUsage};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use deepseek_client::{ToolCall, ToolHandler};

/// Fake tool handler that returns a canned response.
struct FakeToolHandler {
    calls: Mutex<Vec<ToolCall>>,
    response: String,
}

impl FakeToolHandler {
    fn new(response: &str) -> Self {
        Self { calls: Mutex::new(Vec::new()), response: response.to_string() }
    }
}

impl ToolHandler for FakeToolHandler {
    fn execute(&self, call: &ToolCall) -> Result<String, String> {
        self.calls.lock().unwrap().push(call.clone());
        Ok(self.response.clone())
    }
}

/// Mount a mock that serves a sequence of JSON responses, one per request.
/// Falls back to the last response once the sequence is exhausted.
async fn mount_sequence(server: &MockServer, responses: Vec<serde_json::Value>) {
    let counter = Arc::new(Mutex::new(0usize));
    let responses = Arc::new(responses);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(move |_req: &Request| {
            let mut c = counter.lock().unwrap();
            let idx = *c;
            if idx < responses.len() {
                *c += 1;
            }
            let resp_idx = idx.min(responses.len() - 1);
            ResponseTemplate::new(200).set_body_json(responses[resp_idx].clone())
        })
        .mount(server)
        .await;
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct DeepSeekOrderWorld {
    server:    Option<MockServer>,
    handler:   Option<ClaudeCodeHandler>,
    result:    String,
    log_path:  PathBuf,
    _temp_dir: Option<tempfile::TempDir>,
}

impl std::fmt::Debug for DeepSeekOrderWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeepSeekOrderWorld")
            .field("log_path", &self.log_path)
            .field("result", &self.result)
            .finish()
    }
}

impl Default for DeepSeekOrderWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("deepseek_tokens.log");
        Self {
            server:    None,
            handler:   None,
            result:    String::new(),
            log_path,
            _temp_dir: Some(dir),
        }
    }
}

fn ensure_skill_files() {
    // Skill files are copied into the test image by the Dockerfile.
    // If they're missing (legacy image), create minimal stubs so the
    // handler can still assemble a system prompt.
    let dir = std::path::Path::new("/app/.claude/commands");
    if dir.join("claudito.md").exists() && dir.join("search.md").exists() {
        return;
    }
    std::fs::create_dir_all(dir).ok();
    std::fs::write(dir.join("claudito.md"), "claudito base prompt").ok();
    std::fs::write(dir.join("search.md"), "search skill prompt").ok();
}

async fn mount_ok_reply(server: &MockServer, content: &str, input_tokens: u64, output_tokens: u64) {
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": content }
            }],
            "usage": {
                "prompt_tokens": input_tokens,
                "completion_tokens": output_tokens,
                "total_tokens": input_tokens + output_tokens,
            }
        })))
        .mount(server)
        .await;
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given(regex = r#"^a DeepSeek backend that replies "(.+)"$"#)]
async fn given_reply(world: &mut DeepSeekOrderWorld, content: String) {
    ensure_skill_files();
    let server = MockServer::start().await;
    mount_ok_reply(&server, &content, 0, 0).await;
    world.server = Some(server);
}

#[given(regex = r#"^a DeepSeek backend that replies "(.+)" with input_tokens=(\d+) and output_tokens=(\d+)$"#)]
async fn given_reply_with_tokens(world: &mut DeepSeekOrderWorld, content: String, input: u64, output: u64) {
    ensure_skill_files();
    let server = MockServer::start().await;
    mount_ok_reply(&server, &content, input, output).await;
    world.server = Some(server);
}

#[given("a DeepSeek backend that returns HTTP 500")]
async fn given_http_500(world: &mut DeepSeekOrderWorld) {
    ensure_skill_files();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a DeepSeek backend that returns malformed JSON")]
async fn given_malformed_json(world: &mut DeepSeekOrderWorld) {
    ensure_skill_files();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a backend that always returns session_id None")]
fn given_stateless_backend(world: &mut DeepSeekOrderWorld) {
    struct StatelessBackend;
    impl ClaudeBackend for StatelessBackend {
        fn query(&self, _messages: &[ChatMessage]) -> Result<TokenUsage, String> {
            Ok(TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                cache_read_input_tokens: 0,
                cache_creation_input_tokens: 0,
                total_cost_usd: 0.0,
                session_id: None,
                result: "ok".into(),
            })
        }
    }
    world.handler = Some(ClaudeCodeHandler::new(
        Arc::new(StatelessBackend),
        world.log_path.clone(),
    ));
}

#[given("a DeepSeek handler pointed at the mock")]
fn given_handler(world: &mut DeepSeekOrderWorld) {
    let uri = world.server.as_ref().expect("mock server not started").uri();
    let backend = voice_assistant::infrastructure::claude_handler::DeepSeekBackend::with_base_url(
        uri, "test-key".into(), "deepseek-chat".into(),
    );
    world.handler = Some(ClaudeCodeHandler::new(Arc::new(backend), world.log_path.clone()));
}

#[given(regex = r#"^a tool-backed DeepSeek backend that replies "(.+)"$"#)]
async fn given_tool_backend_reply(world: &mut DeepSeekOrderWorld, content: String) {
    ensure_skill_files();
    let server = MockServer::start().await;
    mount_ok_reply(&server, &content, 0, 0).await;
    let uri = server.uri();
    world.server = Some(server);
    // Build the handler immediately with a fake tool handler attached.
    let handler = FakeToolHandler::new("search result");
    let backend = voice_assistant::infrastructure::claude_handler::DeepSeekBackend::with_base_url(
        uri, "test-key".into(), "deepseek-chat".into(),
    )
    .with_tools(Box::new(handler));
    world.handler = Some(ClaudeCodeHandler::new(Arc::new(backend), world.log_path.clone()));
}

#[given(regex = r#"^a tool-backed DeepSeek backend that replies with a tool call then "(.+)"$"#)]
async fn given_tool_backend_with_tool_call(world: &mut DeepSeekOrderWorld, final_answer: String) {
    ensure_skill_files();
    let server = MockServer::start().await;
    mount_sequence(&server, vec![
        // Round 1: tool call
        serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "web_search",
                            "arguments": "{\"query\":\"capital of France\"}"
                        }
                    }]
                }
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
        }),
        // Round 2: text response
        serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": final_answer }
            }],
            "usage": { "prompt_tokens": 15, "completion_tokens": 8 }
        }),
    ]).await;
    let uri = server.uri();
    world.server = Some(server);
    // Build the handler immediately with a fake tool handler attached.
    let handler = FakeToolHandler::new("search result");
    let backend = voice_assistant::infrastructure::claude_handler::DeepSeekBackend::with_base_url(
        uri, "test-key".into(), "deepseek-chat".into(),
    )
    .with_tools(Box::new(handler));
    world.handler = Some(ClaudeCodeHandler::new(Arc::new(backend), world.log_path.clone()));
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when(regex = r#"^the handler handles "(.+)"$"#)]
fn when_handle(world: &mut DeepSeekOrderWorld, order: String) {
    if world.handler.is_none() {
        given_handler(world);
    }
    let handler = world.handler.as_ref().expect("handler not built");
    world.result = handler.handle(&order);
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then(regex = r#"^the return value is "(.+)"$"#)]
fn then_return_value(world: &mut DeepSeekOrderWorld, expected: String) {
    assert_eq!(world.result, expected);
}

#[then("the token log file exists")]
fn then_log_exists(world: &mut DeepSeekOrderWorld) {
    assert!(world.log_path.exists(), "log file should exist at {:?}", world.log_path);
}

#[then(regex = r#"^the log line contains "(.+)"$"#)]
fn then_log_contains(world: &mut DeepSeekOrderWorld, needle: String) {
    let content = std::fs::read_to_string(&world.log_path).unwrap_or_default();
    assert!(
        content.contains(&needle),
        "log should contain \"{needle}\", got:\n{content}"
    );
}

#[then("the return value is an error message")]
fn then_error_message(world: &mut DeepSeekOrderWorld) {
    assert_eq!(world.result, "No tienes tokens disponibles. Por favor, revisa tu configuración.");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main(flavor = "current_thread")]
async fn main() {
    DeepSeekOrderWorld::run("features/deepseek_order_handler.feature").await;
}
