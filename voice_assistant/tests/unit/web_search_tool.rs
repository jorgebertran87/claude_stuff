use cucumber::{given, then, when, World};
use deepseek_client::{ToolCall, ToolHandler};
use serde_json::json;
use std::sync::Arc;
use voice_assistant::infrastructure::web_search::DuckDuckGoSearchTool;

// Re-export needed because ToolHandler is in deepseek_client
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ──────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct WebSearchWorld {
    search_tool: Option<DuckDuckGoSearchTool>,
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

// ── DuckDuckGo Lite HTML fixture ──────────────────────────────────────────────

fn search_results_html() -> String {
    r#"<!DOCTYPE html>
<html>
<body>
<div class="filters"></div>
<table>
  <tr>
    <td><a href="https://www.rust-lang.org/" rel="nofollow">Rust Programming Language</a></td>
  </tr>
  <tr>
    <td class="result-snippet">A language empowering everyone to build reliable and efficient software.</td>
  </tr>
</table>
<table>
  <tr>
    <td><a href="https://en.wikipedia.org/wiki/Rust_(programming_language)" rel="nofollow">Rust (programming language) - Wikipedia</a></td>
  </tr>
  <tr>
    <td class="result-snippet">Rust is a general-purpose programming language emphasizing performance, type safety, and concurrency.</td>
  </tr>
</table>
</body>
</html>"#.to_string()
}

fn no_results_html() -> String {
    r#"<!DOCTYPE html>
<html>
<body>
<div class="no-results">No results found.</div>
</body>
</html>"#.to_string()
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given(regex = r#"^the DuckDuckGo Lite API returns search results for "(.+)"$"#)]
async fn given_search_results(world: &mut WebSearchWorld, _query: String) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/lite/"))
        .and(query_param("q", _query.as_str()))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(search_results_html())
            .insert_header("Content-Type", "text/html"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given(regex = r#"^the DuckDuckGo Lite API returns no results for "(.+)"$"#)]
async fn given_no_results(world: &mut WebSearchWorld, _query: String) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/lite/"))
        .and(query_param("q", _query.as_str()))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(no_results_html())
            .insert_header("Content-Type", "text/html"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("the DuckDuckGo Lite API returns HTTP 500")]
async fn given_http_500(world: &mut WebSearchWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/lite/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when(regex = r#"^the web_search tool executes with query "(.+)"$"#)]
fn when_execute_search(world: &mut WebSearchWorld, query: String) {
    let server_uri = world.server.as_ref().expect("mock server not started").uri();
    // Point the tool at our mock server instead of real DuckDuckGo
    let tool = DuckDuckGoSearchTool::with_base_url(server_uri);
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
