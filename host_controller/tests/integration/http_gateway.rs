use cucumber::{given, then, when, World};
use host_controller::telegram::{http::HttpTelegramGateway, TelegramGateway, TelegramUpdate};
use serde_json::{json, Value};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "TESTTOKEN";

// ── World ────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct GatewayWorld {
    // MockServer must be kept alive so the mock stays mounted during the test.
    server: Option<MockServer>,
    gateway: Option<HttpTelegramGateway>,
    fetched: Option<Vec<TelegramUpdate>>,
}

impl std::fmt::Debug for GatewayWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayWorld").field("fetched", &self.fetched).finish()
    }
}

impl Default for GatewayWorld {
    fn default() -> Self {
        Self { server: None, gateway: None, fetched: None }
    }
}

impl GatewayWorld {
    fn server(&self) -> &MockServer {
        self.server.as_ref().expect("no mock server started")
    }
}

fn get_updates_path() -> String {
    format!("/bot{TOKEN}/getUpdates")
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock Telegram API returning a message "(.*)" with update id (\d+) from chat (\d+)$"#)]
async fn given_mock_message(world: &mut GatewayWorld, text: String, id: i64, chat: i64) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(get_updates_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": [{
                "update_id": id,
                "message": { "chat": { "id": chat }, "text": text }
            }]
        })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Telegram API that only answers a long-poll at offset 5")]
async fn given_mock_longpoll(world: &mut GatewayWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(get_updates_path()))
        .and(query_param("offset", "5"))
        .and(query_param("timeout", "30"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": [{
                "update_id": 5,
                "message": { "chat": { "id": 1 }, "text": "ok" }
            }]
        })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Telegram API returning an update with no message")]
async fn given_mock_no_message(world: &mut GatewayWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(get_updates_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": [{ "update_id": 9 }]
        })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Telegram API that returns HTTP 500")]
async fn given_mock_500(world: &mut GatewayWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(get_updates_path()))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Telegram API accepting messages")]
async fn given_mock_accepting(world: &mut GatewayWorld) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("an http gateway for that API")]
fn given_gateway(world: &mut GatewayWorld) {
    let uri = world.server().uri();
    world.gateway = Some(HttpTelegramGateway::with_base_url(uri, TOKEN.to_string()));
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when(regex = r"^I fetch updates from offset (\d+)$")]
async fn when_fetch(world: &mut GatewayWorld, offset: i64) {
    let updates = world.gateway.as_ref().expect("no gateway").fetch_updates(offset).await;
    world.fetched = Some(updates);
}

#[when(regex = r#"^I post "(.*)" to chat (\d+)$"#)]
async fn when_post(world: &mut GatewayWorld, text: String, chat: i64) {
    world.gateway.as_ref().expect("no gateway").post_message(chat, &text).await;
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then("one update is returned")]
fn then_one(world: &mut GatewayWorld) {
    let f = world.fetched.as_ref().expect("no fetch was made");
    assert_eq!(f.len(), 1, "expected one update, got: {f:?}");
}

#[then("no updates are returned")]
fn then_none(world: &mut GatewayWorld) {
    let f = world.fetched.as_ref().expect("no fetch was made");
    assert!(f.is_empty(), "expected no updates, got: {f:?}");
}

#[then(regex = r#"^the update has id (\d+), chat (\d+), and text "(.*)"$"#)]
fn then_fields(world: &mut GatewayWorld, id: i64, chat: i64, text: String) {
    let u = world
        .fetched
        .as_ref()
        .expect("no fetch was made")
        .first()
        .expect("no update returned");
    assert_eq!(u.update_id, id, "update_id mismatch");
    assert_eq!(u.chat_id, chat, "chat_id mismatch");
    assert_eq!(u.text, text, "text mismatch");
}

#[then(regex = r#"^the API received a message to chat (\d+) containing "(.*)"$"#)]
async fn then_received(world: &mut GatewayWorld, chat: i64, needle: String) {
    let requests = world
        .server()
        .received_requests()
        .await
        .expect("request recording disabled");
    let found = requests.iter().any(|req| {
        req.url.path().ends_with("/sendMessage")
            && serde_json::from_slice::<Value>(&req.body)
                .ok()
                .map(|b| {
                    b["chat_id"] == json!(chat)
                        && b["text"].as_str().is_some_and(|t| t.contains(&needle))
                })
                .unwrap_or(false)
    });
    assert!(found, "no sendMessage to chat {chat} containing {needle:?} ({} request(s) seen)", requests.len());
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    GatewayWorld::run("features/http_gateway.feature").await;
}
