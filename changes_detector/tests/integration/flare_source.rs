use changes_detector::source::flare::{FetchMode, FlareSolverSource};
use changes_detector::source::Source;
use cucumber::{given, then, when, World};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct FlareWorld {
    // MockServer must be kept alive so the mock remains mounted during the test.
    server:       Option<MockServer>,
    server_uri:   String,
    source:       Option<FlareSolverSource>,
    fetch_result: Option<Result<String, String>>,
}

impl std::fmt::Debug for FlareWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlareWorld")
            .field("server_uri", &self.server_uri)
            .finish()
    }
}

impl Default for FlareWorld {
    fn default() -> Self {
        Self {
            server:       None,
            server_uri:   String::new(),
            source:       None,
            fetch_result: None,
        }
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

/// Mount a mock FlareSolverr that returns an HTML page containing the
/// given CSS element (class selector) with the specified text content.
#[given(regex = r#"^a mock FlareSolverr returning a page with element "([^"]+)" containing "([^"]+)"$"#)]
async fn given_mock_with_element(world: &mut FlareWorld, selector: String, text: String) {
    let (tag, class) = parse_selector(&selector);
    let html = format!(
        r#"<html><body><{tag} class="{class}">{text}</{tag}></body></html>"#,
    );

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "solution": { "response": html }
            })),
        )
        .mount(&server)
        .await;

    world.server_uri = server.uri();
    world.server     = Some(server);
}

#[given("a mock FlareSolverr that returns HTTP 500")]
async fn given_mock_error(world: &mut FlareWorld) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    world.server_uri = server.uri();
    world.server     = Some(server);
}

#[given(regex = r#"^a FlareSolverSource in content mode targeting selector "([^"]+)"$"#)]
fn given_source_content(world: &mut FlareWorld, selector: String) {
    world.source = Some(FlareSolverSource::new(
        "http://example.com".into(),
        Some(selector),
        FetchMode::Content,
        world.server_uri.clone(),
    ));
}

#[given(regex = r#"^a FlareSolverSource in existence mode targeting selector "([^"]+)"$"#)]
fn given_source_existence(world: &mut FlareWorld, selector: String) {
    world.source = Some(FlareSolverSource::new(
        "http://example.com".into(),
        Some(selector),
        FetchMode::Existence,
        world.server_uri.clone(),
    ));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when("I fetch from the source")]
async fn when_fetch(world: &mut FlareWorld) {
    let source = world.source.as_ref().unwrap();
    world.fetch_result = Some(source.fetch().await.map_err(|e| e.to_string()));
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the fetch succeeds")]
fn then_ok(world: &mut FlareWorld) {
    let result = world.fetch_result.as_ref().unwrap();
    assert!(result.is_ok(), "expected fetch to succeed, got: {:?}", result);
}

#[then("the fetch fails")]
fn then_err(world: &mut FlareWorld) {
    let result = world.fetch_result.as_ref().unwrap();
    assert!(
        result.is_err(),
        "expected fetch to fail, but it succeeded with: {:?}",
        result
    );
}

#[then(regex = r#"^the result contains "([^"]+)"$"#)]
fn then_contains(world: &mut FlareWorld, needle: String) {
    let text = world
        .fetch_result
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_or_else(|e| panic!("fetch failed: {e}"));
    assert!(
        text.contains(&needle),
        "expected result to contain \"{needle}\", got: {text}"
    );
}

#[then(regex = r#"^the result is "([^"]+)"$"#)]
fn then_equals(world: &mut FlareWorld, expected: String) {
    let text = world
        .fetch_result
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_or_else(|e| panic!("fetch failed: {e}"));
    assert_eq!(text, &expected, "result mismatch");
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse a simple "tag.class" CSS selector into (tag, class).
/// e.g. "div.content" → ("div", "content")
fn parse_selector(selector: &str) -> (&str, &str) {
    if let Some((tag, class)) = selector.split_once('.') {
        (tag, class)
    } else {
        (selector, "")
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    FlareWorld::run("features/flare_source.feature").await;
}
