use cucumber::{given, then, when, World};
use deepseek_client::{chat_with_tools, ChatMessage, ToolCall, ToolDefinition, ToolHandler};
use serde_json::json;
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ── Fake tool handler ──────────────────────────────────────────────────────────

struct FakeToolHandler {
    calls: Mutex<Vec<ToolCall>>,
    response: String,
}

impl FakeToolHandler {
    fn new(response: &str) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            response: response.to_string(),
        }
    }
}

impl ToolHandler for FakeToolHandler {
    fn execute(&self, call: &ToolCall) -> Result<String, String> {
        self.calls.lock().unwrap().push(call.clone());
        Ok(self.response.clone())
    }
}

struct ErrorToolHandler;

impl ToolHandler for ErrorToolHandler {
    fn execute(&self, _call: &ToolCall) -> Result<String, String> {
        Err("Search engine unavailable".to_string())
    }
}

// ── Tool definitions ───────────────────────────────────────────────────────────

fn web_search_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_search".into(),
        description: "Search the web".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "The search query"}
            },
            "required": ["query"]
        }),
    }
}

fn url_fetch_tool() -> ToolDefinition {
    ToolDefinition {
        name: "url_fetch".into(),
        description: "Fetch a URL".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "The URL to fetch"}
            },
            "required": ["url"]
        }),
    }
}

// ── World ──────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct ToolCallingWorld {
    server: Option<MockServer>,
    result: Option<Result<String, String>>,
    tool_calls: Vec<ToolCall>,
    response: String,
}

impl std::fmt::Debug for ToolCallingWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolCallingWorld")
            .field("result", &self.result)
            .field("tool_calls", &self.tool_calls)
            .finish()
    }
}

impl Default for ToolCallingWorld {
    fn default() -> Self {
        Self {
            server: None,
            result: None,
            tool_calls: Vec::new(),
            response: String::new(),
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn messages() -> Vec<ChatMessage> {
    vec![
        ChatMessage::new("system", "You are a helpful assistant."),
        ChatMessage::new("user", "What is the capital of France?"),
    ]
}

fn tools() -> Vec<ToolDefinition> {
    vec![web_search_tool(), url_fetch_tool()]
}

/// Mount a mock that serves a sequence of responses, one per request.
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
            // Always return the last response once we've exhausted the sequence
            let resp_idx = idx.min(responses.len() - 1);
            ResponseTemplate::new(200).set_body_json(responses[resp_idx].clone())
        })
        .mount(server)
        .await;
}

fn text_response(content: &str, input: u64, output: u64) -> serde_json::Value {
    json!({
        "choices": [{
            "message": { "role": "assistant", "content": content }
        }],
        "usage": { "prompt_tokens": input, "completion_tokens": output }
    })
}

fn tool_call_response(calls: serde_json::Value, input: u64, output: u64) -> serde_json::Value {
    json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": calls
            }
        }],
        "usage": { "prompt_tokens": input, "completion_tokens": output }
    })
}

fn single_web_search_tool_call() -> serde_json::Value {
    json!([{
        "id": "call_1",
        "type": "function",
        "function": {
            "name": "web_search",
            "arguments": "{\"query\":\"capital of France\"}"
        }
    }])
}

fn two_web_search_tool_calls() -> serde_json::Value {
    json!([
        {
            "id": "call_1",
            "type": "function",
            "function": {
                "name": "web_search",
                "arguments": "{\"query\":\"capital of France\"}"
            }
        },
        {
            "id": "call_2",
            "type": "function",
            "function": {
                "name": "web_search",
                "arguments": "{\"query\":\"France population\"}"
            }
        }
    ])
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given("a DeepSeek API that returns a text response")]
async fn given_text_response(world: &mut ToolCallingWorld) {
    let server = MockServer::start().await;
    mount_sequence(&server, vec![
        text_response("Paris is the capital of France", 10, 5),
    ]).await;
    world.server = Some(server);
}

#[given("a DeepSeek API that returns a web_search tool call")]
async fn given_single_tool_call(world: &mut ToolCallingWorld) {
    let server = MockServer::start().await;
    mount_sequence(&server, vec![
        tool_call_response(single_web_search_tool_call(), 10, 5),
        text_response("The capital of France is Paris.", 15, 8),
    ]).await;
    world.server = Some(server);
}

#[given("a DeepSeek API that returns two tool calls in one response")]
async fn given_parallel_tool_calls(world: &mut ToolCallingWorld) {
    let server = MockServer::start().await;
    mount_sequence(&server, vec![
        tool_call_response(two_web_search_tool_calls(), 10, 5),
        text_response("Here are the results.", 15, 8),
    ]).await;
    world.server = Some(server);
}

#[given("a DeepSeek API that returns a tool call then a text response")]
async fn given_two_rounds(world: &mut ToolCallingWorld) {
    let server = MockServer::start().await;
    mount_sequence(&server, vec![
        tool_call_response(single_web_search_tool_call(), 10, 5),
        text_response("The capital of France is Paris.", 20, 8),
    ]).await;
    world.server = Some(server);
}

#[given("a DeepSeek API that always returns a tool call")]
async fn given_always_tool_call(world: &mut ToolCallingWorld) {
    let server = MockServer::start().await;
    // Mount enough responses to exceed MAX_TOOL_ROUNDS (50)
    let responses: Vec<serde_json::Value> = (0..51).map(|_| {
        tool_call_response(single_web_search_tool_call(), 1, 1)
    }).collect();
    mount_sequence(&server, responses).await;
    world.server = Some(server);
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when("chat_with_tools is called with a FakeToolHandler returning \"search result\"")]
fn when_chat_with_fake_handler(world: &mut ToolCallingWorld) {
    let server = world.server.as_ref().expect("mock server not started");
    let handler = FakeToolHandler::new("search result");
    let result = chat_with_tools(
        &server.uri(),
        "test-key",
        "deepseek-chat",
        &messages(),
        &tools(),
        &handler,
        None,
    );
    world.tool_calls = handler.calls.lock().unwrap().clone();
    match result {
        Ok(resp) => {
            let content = resp.content.clone();
            world.response = resp.content;
            world.result = Some(Ok(content));
        }
        Err(e) => world.result = Some(Err(e)),
    }
}

#[when("chat_with_tools is called with an ErrorToolHandler")]
fn when_chat_with_error_handler(world: &mut ToolCallingWorld) {
    let server = world.server.as_ref().expect("mock server not started");
    let handler = ErrorToolHandler;
    let result = chat_with_tools(
        &server.uri(),
        "test-key",
        "deepseek-chat",
        &messages(),
        &tools(),
        &handler,
        None,
    );
    match result {
        Ok(resp) => world.result = Some(Ok(resp.content)),
        Err(e) => world.result = Some(Err(e)),
    }
}

#[when("chat_with_tools is called with a handler returning \"ok\"")]
fn when_chat_ok_handler(world: &mut ToolCallingWorld) {
    let server = world.server.as_ref().expect("mock server not started");
    let handler = FakeToolHandler::new("ok");
    let result = chat_with_tools(
        &server.uri(),
        "test-key",
        "deepseek-chat",
        &messages(),
        &tools(),
        &handler,
        None,
    );
    world.tool_calls = handler.calls.lock().unwrap().clone();
    match result {
        Ok(resp) => world.result = Some(Ok(resp.content)),
        Err(e) => world.result = Some(Err(e)),
    }
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then("the response content is not empty")]
fn then_content_not_empty(world: &mut ToolCallingWorld) {
    match &world.result {
        Some(Ok(content)) => assert!(!content.is_empty(), "expected non-empty content"),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then("no tool calls were made")]
fn then_no_tool_calls(world: &mut ToolCallingWorld) {
    assert!(
        world.tool_calls.is_empty(),
        "expected no tool calls, got {}",
        world.tool_calls.len()
    );
}

#[then("one tool call was made")]
fn then_one_tool_call(world: &mut ToolCallingWorld) {
    assert_eq!(world.tool_calls.len(), 1, "expected 1 tool call, got {}", world.tool_calls.len());
}

#[then("two tool calls were made")]
fn then_two_tool_calls(world: &mut ToolCallingWorld) {
    assert_eq!(world.tool_calls.len(), 2, "expected 2 tool calls, got {}", world.tool_calls.len());
}

#[then("the response content is \"Paris is the capital of France\"")]
fn then_content_paris(world: &mut ToolCallingWorld) {
    match &world.result {
        Some(Ok(content)) => assert_eq!(content, "Paris is the capital of France"),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then("the response is a final answer")]
fn then_final_answer(world: &mut ToolCallingWorld) {
    match &world.result {
        Some(Ok(_)) => {}
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then("an error is returned")]
fn then_error_returned(world: &mut ToolCallingWorld) {
    match &world.result {
        Some(Err(_)) => {}
        Some(Ok(content)) => panic!("expected Err, got Ok: {content}"),
        None => panic!("no result set"),
    }
}

#[then(regex = r#"^the tool call was for "(.+)"$"#)]
fn then_tool_call_name(world: &mut ToolCallingWorld, name: String) {
    assert!(
        world.tool_calls.iter().any(|tc| tc.name == name),
        "expected a tool call for \"{name}\", got calls: {:?}",
        world.tool_calls.iter().map(|tc| &tc.name).collect::<Vec<_>>()
    );
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[tokio::main(flavor = "current_thread")]
async fn main() {
    ToolCallingWorld::run("features/deepseek_tool_calling.feature").await;
}
