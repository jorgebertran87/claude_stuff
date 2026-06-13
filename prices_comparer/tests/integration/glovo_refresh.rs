use cucumber::{given, then, when, World};
use prices_comparer::source::glovo_refresh::{
    GlovoRefresher, RefreshCreds, RefreshError, RefreshStore,
};
use prices_comparer::token_store::TokenStore;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN_FILE: &str = "glovo_token";
const REFRESH_FILE: &str = "glovo_refresh.json";

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct RefreshWorld {
    // MockServer and TempDir must outlive the test.
    server: Option<MockServer>,
    dir: tempfile::TempDir,
    refresher: Option<GlovoRefresher>,
    result: Option<Result<(), RefreshError>>,
}

impl std::fmt::Debug for RefreshWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefreshWorld").field("result", &self.result).finish()
    }
}

impl Default for RefreshWorld {
    fn default() -> Self {
        Self {
            server: None,
            dir: tempfile::tempdir().unwrap(),
            refresher: None,
            result: None,
        }
    }
}

impl RefreshWorld {
    fn token_store(&self) -> TokenStore {
        TokenStore::new(self.dir.path().join(TOKEN_FILE))
    }

    fn refresh_store(&self) -> RefreshStore {
        RefreshStore::new(self.dir.path().join(REFRESH_FILE))
    }

    async fn ensure_server(&mut self) -> &MockServer {
        if self.server.is_none() {
            self.server = Some(MockServer::start().await);
        }
        self.server.as_ref().unwrap()
    }

    /// POST bodies the mock received on /oauth/refresh.
    async fn refresh_bodies(&self) -> Vec<String> {
        self.server
            .as_ref()
            .expect("no server")
            .received_requests()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.url.path() == "/oauth/refresh")
            .map(|r| String::from_utf8_lossy(&r.body).to_string())
            .collect()
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a stored refresh token "([^"]+)" and device urn "([^"]+)"$"#)]
fn given_stored(world: &mut RefreshWorld, token: String, urn: String) {
    world
        .refresh_store()
        .save(&RefreshCreds { refresh_token: token, device_urn: urn })
        .unwrap();
}

#[given("no refresh token is configured")]
fn given_unconfigured(_world: &mut RefreshWorld) {}

#[given(regex = r#"^a mock Glovo auth that issues access token "([^"]+)" and refresh token "([^"]+)"$"#)]
async fn given_mock_issues(world: &mut RefreshWorld, access: String, refresh: String) {
    let server = world.ensure_server().await;
    Mock::given(method("POST"))
        .and(path("/oauth/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accessToken": access,
            "refreshToken": refresh,
            "expiresIn": 1200,
            "tokenType": "bearer"
        })))
        .mount(server)
        .await;
}

#[given("a mock Glovo auth that rejects the refresh token")]
async fn given_mock_rejects(world: &mut RefreshWorld) {
    let server = world.ensure_server().await;
    Mock::given(method("POST"))
        .and(path("/oauth/refresh"))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid_token"))
        .mount(server)
        .await;
}

#[given("a refresher pointed at the mock")]
async fn given_refresher(world: &mut RefreshWorld) {
    let uri = world.ensure_server().await.uri();
    let tokens = world.token_store();
    let creds = world.refresh_store();
    world.refresher = Some(GlovoRefresher::new(uri, tokens, creds));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when("I refresh")]
async fn when_refresh(world: &mut RefreshWorld) {
    let refresher = world.refresher.as_ref().expect("no refresher");
    world.result = Some(refresher.refresh().await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the access token in the token store is "([^"]+)"$"#)]
fn then_access(world: &mut RefreshWorld, expected: String) {
    assert_eq!(world.token_store().current(), Some(expected), "access token mismatch");
}

#[then(regex = r#"^the stored refresh token is "([^"]+)"$"#)]
fn then_stored_refresh(world: &mut RefreshWorld, expected: String) {
    let creds = world.refresh_store().current().expect("no creds stored");
    assert_eq!(creds.refresh_token, expected, "stored refresh token mismatch");
}

#[then(regex = r#"^the last refresh request sent the token "([^"]+)"$"#)]
async fn then_last_request(world: &mut RefreshWorld, expected: String) {
    let bodies = world.refresh_bodies().await;
    let last = bodies.last().unwrap_or_else(|| panic!("no refresh requests were made"));
    assert!(
        last.contains(&format!("\"refreshToken\":\"{expected}\"")),
        "expected last request to send token {expected:?}, got body: {last}"
    );
}

#[then("the refresh reports the token was rejected")]
fn then_rejected(world: &mut RefreshWorld) {
    assert_eq!(world.result, Some(Err(RefreshError::Rejected)), "expected Rejected");
}

#[then("no refresh request was made")]
async fn then_no_request(world: &mut RefreshWorld) {
    let bodies = world.refresh_bodies().await;
    assert!(bodies.is_empty(), "expected no refresh requests, got: {bodies:?}");
}

#[then("the token store is still empty")]
fn then_empty(world: &mut RefreshWorld) {
    assert_eq!(world.token_store().current(), None, "expected empty token store");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    RefreshWorld::run("features/glovo_refresh.feature").await;
}
