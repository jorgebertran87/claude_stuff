use cucumber::{given, then, when, World};
use deepseek_client::{ToolCall, ToolHandler};
use serde_json::json;
use voice_assistant::infrastructure::url_fetcher::UrlFetcherTool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ──────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct UrlFetchWorld {
    tool: Option<UrlFetcherTool>,
    result: Option<Result<String, String>>,
    server: Option<MockServer>,
}

impl std::fmt::Debug for UrlFetchWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UrlFetchWorld")
            .field("result", &self.result)
            .finish()
    }
}

impl Default for UrlFetchWorld {
    fn default() -> Self {
        Self {
            tool: None,
            result: None,
            server: None,
        }
    }
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given(regex = r#"^a URL "(.+)" that returns plain text "(.+)"$"#)]
async fn given_plain_text(world: &mut UrlFetchWorld, url_path: String, body: String) {
    let server = MockServer::start().await;
    let path_only = url_path.strip_prefix("https://example.com").unwrap_or(&url_path).to_string();
    Mock::given(method("GET"))
        .and(path(path_only))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(body)
            .insert_header("Content-Type", "text/plain"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given(regex = r#"^a URL "(.+)" that returns HTML with body text "(.+)"$"#)]
async fn given_html_page(world: &mut UrlFetchWorld, url_path: String, body_text: String) {
    let server = MockServer::start().await;
    let path_only = url_path.strip_prefix("https://example.com").unwrap_or(&url_path).to_string();
    let html = format!(
        "<!DOCTYPE html><html><head><title>Test</title></head><body><h1>{}</h1><p>Some content.</p></body></html>",
        body_text
    );
    Mock::given(method("GET"))
        .and(path(path_only))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(html)
            .insert_header("Content-Type", "text/html"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given(regex = r#"^a URL "(.+)" that returns HTTP 404$"#)]
async fn given_http_404(world: &mut UrlFetchWorld, url_path: String) {
    let server = MockServer::start().await;
    let path_only = url_path.strip_prefix("https://example.com").unwrap_or(&url_path).to_string();
    Mock::given(method("GET"))
        .and(path(path_only))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given(regex = r#"^a URL "(.+)" that is unreachable$"#)]
fn given_unreachable(_world: &mut UrlFetchWorld, _url: String) {
    // No mock server — the tool will try to connect and fail
}

#[given(regex = r#"^a URL "(.+)" that returns (\d+) characters of text$"#)]
async fn given_large_response(world: &mut UrlFetchWorld, url_path: String, count: usize) {
    let server = MockServer::start().await;
    let path_only = url_path.strip_prefix("https://example.com").unwrap_or(&url_path).to_string();
    let body = "x".repeat(count);
    Mock::given(method("GET"))
        .and(path(path_only))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(body)
            .insert_header("Content-Type", "text/plain"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when(regex = r#"^the url_fetch tool executes with url "(.+)"$"#)]
fn when_fetch_url(world: &mut UrlFetchWorld, url: String) {
    let tool = UrlFetcherTool::new();

    // Rewrite the URL to point at the mock server, or use original for unreachable test
    let actual_url = if let Some(ref server) = world.server {
        let path = url.strip_prefix("https://example.com").unwrap_or(&url);
        format!("{}{}", server.uri(), path)
    } else {
        url
    };

    let call = ToolCall {
        id: "call_1".into(),
        name: "url_fetch".into(),
        arguments: json!({"url": actual_url}),
    };
    world.result = Some(tool.execute(&call));
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then(regex = r#"^the result contains "(.+)"$"#)]
fn then_result_contains(world: &mut UrlFetchWorld, needle: String) {
    match &world.result {
        Some(Ok(content)) => assert!(
            content.contains(&needle),
            "expected result to contain \"{needle}\", got: {content}"
        ),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then(regex = r#"^the result does not contain "(.+)"$"#)]
fn then_result_does_not_contain(world: &mut UrlFetchWorld, needle: String) {
    match &world.result {
        Some(Ok(content)) => assert!(
            !content.contains(&needle),
            "expected result to NOT contain \"{needle}\", got: {content}"
        ),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

#[then("the result contains an error message")]
fn then_has_error(world: &mut UrlFetchWorld) {
    match &world.result {
        Some(Ok(content)) => {
            let lower = content.to_lowercase();
            assert!(
                lower.contains("error") || lower.contains("failed") || lower.contains("unable"),
                "expected error message, got: {content}"
            );
        }
        Some(Err(e)) => assert!(!e.is_empty()),
        None => panic!("no result set"),
    }
}

#[then(regex = r#"^the result is shorter than (\d+) characters$"#)]
fn then_shorter_than(world: &mut UrlFetchWorld, max: usize) {
    match &world.result {
        Some(Ok(content)) => assert!(
            content.len() < max,
            "expected result shorter than {max}, got {} chars",
            content.len()
        ),
        Some(Err(e)) => panic!("expected Ok, got Err: {e}"),
        None => panic!("no result set"),
    }
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[tokio::main(flavor = "current_thread")]
async fn main() {
    UrlFetchWorld::run("features/url_fetch_tool.feature").await;
}
