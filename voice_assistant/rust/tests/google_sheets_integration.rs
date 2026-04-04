use cucumber::{given, when, then, World};

use voice_assistant::infrastructure::google_sheets::{SheetsClient, auth_url};

#[derive(Default, World)]
pub struct SheetsWorld {
    client: Option<Option<SheetsClient>>,
    auth_url_result: Option<Option<String>>,
    fetch_result: Option<Result<String, String>>,
}

impl std::fmt::Debug for SheetsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SheetsWorld")
            .field("auth_url_result", &self.auth_url_result)
            .field("fetch_result", &self.fetch_result)
            .finish()
    }
}

// ── from_env scenarios ──────────────────────────────────────────────────────

#[given("the Google Sheets environment variables are set")]
fn given_env_set(_world: &mut SheetsWorld) {
    assert!(
        std::env::var("GOOGLE_SPREADSHEET_ID").is_ok(),
        "GOOGLE_SPREADSHEET_ID must be set"
    );
}

#[given("the GOOGLE_SPREADSHEET_ID environment variable is unset")]
fn given_spreadsheet_unset(_world: &mut SheetsWorld) {
    unsafe { std::env::remove_var("GOOGLE_SPREADSHEET_ID"); }
}

#[when("SheetsClient::from_env is called")]
fn when_from_env(world: &mut SheetsWorld) {
    world.client = Some(SheetsClient::from_env());
}

#[then("it returns a valid SheetsClient")]
fn then_valid_client(world: &mut SheetsWorld) {
    assert!(
        world.client.as_ref().unwrap().is_some(),
        "expected Some(SheetsClient)"
    );
}

#[then("it returns None")]
fn then_none(world: &mut SheetsWorld) {
    assert!(
        world.client.as_ref().unwrap().is_none(),
        "expected None"
    );
}

// ── auth_url scenario ───────────────────────────────────────────────────────

#[given("the GOOGLE_CLIENT_ID environment variable is set")]
fn given_client_id_set(_world: &mut SheetsWorld) {
    assert!(
        std::env::var("GOOGLE_CLIENT_ID").is_ok(),
        "GOOGLE_CLIENT_ID must be set"
    );
}

#[when("auth_url is called")]
fn when_auth_url(world: &mut SheetsWorld) {
    world.auth_url_result = Some(auth_url());
}

#[then("the result is a URL containing the client ID")]
fn then_url_has_client_id(world: &mut SheetsWorld) {
    let url = world.auth_url_result.as_ref().unwrap().as_ref()
        .expect("auth_url returned None");
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap();
    assert!(url.contains(&client_id), "URL should contain client_id");
}

#[then(regex = r#"^the result contains "(.+)"$"#)]
fn then_url_contains(world: &mut SheetsWorld, needle: String) {
    let url = world.auth_url_result.as_ref().unwrap().as_ref()
        .expect("auth_url returned None");
    assert!(url.contains(&needle), "URL should contain \"{needle}\"");
}

// ── fetch_as_text scenario ──────────────────────────────────────────────────

#[given("a valid SheetsClient built from environment variables")]
fn given_valid_client(world: &mut SheetsWorld) {
    let client = SheetsClient::from_env().expect("SheetsClient::from_env returned None");
    world.client = Some(Some(client));
}

#[when("fetch_as_text is called")]
fn when_fetch(world: &mut SheetsWorld) {
    let client = world.client.as_ref().unwrap().as_ref().unwrap();
    world.fetch_result = Some(client.fetch_as_text());
}

#[then("the result is a non-empty string containing tabs and newlines")]
fn then_non_empty_tsv(world: &mut SheetsWorld) {
    let text = world.fetch_result.as_ref().unwrap().as_ref()
        .expect("fetch_as_text failed");
    assert!(!text.is_empty(), "fetch result should not be empty");
    assert!(text.contains('\t'), "fetch result should contain tabs");
    assert!(text.contains('\n'), "fetch result should contain newlines");
}

fn main() {
    // Run "None" scenario first (it mutates env), then restore before others.
    // Cucumber runs scenarios concurrently by default, so we serialize them.
    futures::executor::block_on(
        SheetsWorld::cucumber()
            .max_concurrent_scenarios(1)
            .run_and_exit("features/google_sheets_integration.feature"),
    );
}
