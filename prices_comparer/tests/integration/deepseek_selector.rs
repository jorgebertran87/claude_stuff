use cucumber::{given, then, when, World};
use prices_comparer::comparer::ProductSelector;
use prices_comparer::normalizer::DeepSeekProductSelector;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct SelectorWorld {
    // MockServer must be kept alive so the mock remains mounted during the test.
    server: Option<MockServer>,
    selector: Option<DeepSeekProductSelector>,
    candidates: Vec<String>,
    selected: Option<usize>,
}

impl std::fmt::Debug for SelectorWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectorWorld").field("selected", &self.selected).finish()
    }
}

impl Default for SelectorWorld {
    fn default() -> Self {
        Self { server: None, selector: None, candidates: Vec::new(), selected: None }
    }
}

/// Mount a DeepSeek chat-completions mock whose reply carries `content`.
async fn mount_reply(world: &mut SelectorWorld, content: String) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [ { "message": { "role": "assistant", "content": content } } ]
        })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock DeepSeek API that selects candidate (\d+)$"#)]
async fn given_selects(world: &mut SelectorWorld, index: String) {
    mount_reply(world, index).await;
}

#[given(regex = r#"^a mock DeepSeek API that replies "([^"]+)"$"#)]
async fn given_replies(world: &mut SelectorWorld, reply: String) {
    mount_reply(world, reply).await;
}

#[given("a mock DeepSeek API that returns HTTP 500")]
async fn given_http_error(world: &mut SelectorWorld) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a DeepSeek selector pointed at the mock")]
fn given_selector(world: &mut SelectorWorld) {
    let uri = world.server.as_ref().expect("mock server not started").uri();
    world.selector =
        Some(DeepSeekProductSelector::with_base_url(uri, "test-key".into(), "deepseek-chat".into()));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I select for "([^"]+)" among (.+)$"#)]
async fn when_select(world: &mut SelectorWorld, description: String, list: String) {
    world.candidates = list.split(", ").map(|s| s.trim_matches('"').to_string()).collect();
    let selector = world.selector.as_ref().expect("selector not built");
    world.selected = selector.select(&description, &world.candidates).await;
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the chosen candidate is "([^"]+)"$"#)]
fn then_chosen(world: &mut SelectorWorld, expected: String) {
    let i = world.selected.unwrap_or_else(|| panic!("expected a selection, got none"));
    assert_eq!(world.candidates.get(i).map(String::as_str), Some(expected.as_str()), "wrong candidate");
}

#[then("nothing is selected")]
fn then_nothing(world: &mut SelectorWorld) {
    assert_eq!(world.selected, None, "expected no selection, got {:?}", world.selected);
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    SelectorWorld::run("features/deepseek_selector.feature").await;
}
