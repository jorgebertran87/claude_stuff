use cucumber::{given, then, when, World};
use deepseek_client::{ToolCall, ToolHandler};
use serde_json::json;
use voice_assistant::infrastructure::web_search::SearXngSearchTool;

// Re-export needed because ToolHandler is in deepseek_client
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ──────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct WebSearchWorld {
    search_tool: Option<SearXngSearchTool>,
    result: Option<Result<String, String>>,
    server: Option<MockServer>,
}

impl std::fmt::Debug for WebSearchWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSearchWorld")
            .field("result", &self.result)
            .finish()
    }
}

impl Default for WebSearchWorld {
    fn default() -> Self {
        Self {
            search_tool: None,
            result: None,
            server: None,
        }
    }
}

// ── SearXNG JSON fixtures ─────────────────────────────────────────────────────

fn search_results_json() -> serde_json::Value {
    json!({
        "query": "rust programming language",
        "results": [
            {
                "title": "Rust Programming Language",
                "url": "https://www.rust-lang.org/",
                "content": "A language empowering everyone to build reliable and efficient software.",
                "engine": "duckduckgo"
            },
            {
                "title": "Rust (programming language) - Wikipedia",
                "url": "https://en.wikipedia.org/wiki/Rust_(programming_language)",
                "content": "Rust is a general-purpose programming language emphasizing performance, type safety, and concurrency.",
                "engine": "wikipedia"
            }
        ]
    })
}

fn no_results_json() -> serde_json::Value {
    json!({
        "query": "xyznonexistent123",
        "results": []
    })
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given(regex = r#"^the SearXNG API returns search results for "(.+)"$"#)]
async fn given_search_results(world: &mut WebSearchWorld, query: String) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("format", "json"))
        .and(query_param("q", query.as_str()))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(search_results_json())
            .insert_header("Content-Type", "application/json"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given(regex = r#"^the SearXNG API returns no results for "(.+)"$"#)]
async fn given_no_results(world: &mut WebSearchWorld, query: String) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("format", "json"))
        .and(query_param("q", query.as_str()))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(no_results_json())
            .insert_header("Content-Type", "application/json"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("the SearXNG API returns HTTP 500")]
async fn given_http_500(world: &mut WebSearchWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when(regex = r#"^the web_search tool executes with query "(.+)"$"#)]
fn when_execute_search(world: &mut WebSearchWorld, query: String) {
    let server_uri = world.server.as_ref().expect("mock server not started").uri();
    // Point the tool at our mock server instead of real SearXNG.
    let tool = SearXngSearchTool::with_base_url(server_uri);
    let call = ToolCall {
        id: "call_1".into(),
        name: "web_search".into(),
        arguments: json!({"query": query}),
    };
    world.result = Some(tool.execute(&call));
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then(regex = r#"^the result contains "(.+)"$"#)]
fn then_result_contains(world: &mut WebSearchWorld, needle: String) {
    match &world.result {
        Some(Ok(content)) => assert!(
            content.contains(&needle),
            "expected result to contain \"{needle}\", got: {content}"
        ),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then("the result contains a URL starting with \"http\"")]
fn then_result_has_url(world: &mut WebSearchWorld) {
    match &world.result {
        Some(Ok(content)) => assert!(
            content.contains("http"),
            "expected result to contain a URL, got: {content}"
        ),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then("the result is not empty")]
fn then_result_not_empty(world: &mut WebSearchWorld) {
    match &world.result {
        Some(Ok(content)) => assert!(!content.is_empty(), "expected non-empty result"),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then("the result does not contain an error message")]
fn then_no_error_message(world: &mut WebSearchWorld) {
    match &world.result {
        Some(Ok(content)) => assert!(
            !content.to_lowercase().contains("error"),
            "result should not contain 'error', got: {content}"
        ),
        Some(Err(_)) => panic!("expected Ok, got Err"),
        None => panic!("no result set"),
    }
}

#[then("the result contains an error message")]
fn then_has_error(world: &mut WebSearchWorld) {
    match &world.result {
        Some(Ok(content)) => assert!(
            content.to_lowercase().contains("error"),
            "result should contain an error message, got: {content}"
        ),
        Some(Err(e)) => assert!(!e.is_empty()),
        None => panic!("no result set"),
    }
}

#[then("no panic occurs")]
fn then_no_panic(_world: &mut WebSearchWorld) {
    // If we got here, no panic occurred during tool execution
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[tokio::main(flavor = "current_thread")]
async fn main() {
    WebSearchWorld::run("features/web_search_tool.feature").await;
}
