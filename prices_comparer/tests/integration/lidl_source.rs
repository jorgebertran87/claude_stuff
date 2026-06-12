use cucumber::{given, then, when, World};
use prices_comparer::comparer::StoreSource;
use prices_comparer::source::lidl::LidlSource;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct LidlWorld {
    // MockServer must be kept alive so the mock remains mounted during the test.
    server: Option<MockServer>,
    source: Option<LidlSource>,
    result: Option<Result<Option<u64>, String>>,
}

impl std::fmt::Debug for LidlWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LidlWorld")
            .field("result", &self.result)
            .finish()
    }
}

impl Default for LidlWorld {
    fn default() -> Self {
        Self { server: None, source: None, result: None }
    }
}

/// Parse a price like "0.99" into cents without going through floats.
fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

fn item(name: &str, price: &str) -> serde_json::Value {
    json!({
        "gridbox": {
            "data": {
                "fullTitle": name,
                "price": { "price": price.parse::<f64>().unwrap() }
            }
        }
    })
}

async fn mount_search(world: &mut LidlWorld, term: &str, items: serde_json::Value) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/q/api/search"))
        .and(query_param("q", term))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "items": items })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock Lidl API where searching "([^"]+)" finds "([^"]+)" at (\d+\.\d+)$"#)]
async fn given_one_hit(world: &mut LidlWorld, term: String, name: String, price: String) {
    mount_search(world, &term, json!([item(&name, &price)])).await;
}

#[given(regex = r#"^a mock Lidl API where searching "([^"]+)" finds "([^"]+)" at (\d+\.\d+) and "([^"]+)" at (\d+\.\d+)$"#)]
async fn given_two_hits(
    world: &mut LidlWorld,
    term: String,
    name_a: String,
    price_a: String,
    name_b: String,
    price_b: String,
) {
    mount_search(world, &term, json!([item(&name_a, &price_a), item(&name_b, &price_b)])).await;
}

#[given(regex = r#"^a mock Lidl API where searching "([^"]+)" finds nothing$"#)]
async fn given_no_hits(world: &mut LidlWorld, term: String) {
    mount_search(world, &term, json!([])).await;
}

#[given("a mock Lidl API that returns HTTP 500")]
async fn given_http_error(world: &mut LidlWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Lidl API that returns invalid JSON")]
async fn given_invalid_json(world: &mut LidlWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a Lidl source pointed at the mock")]
fn given_source(world: &mut LidlWorld) {
    let uri = world.server.as_ref().expect("mock server not started").uri();
    world.source = Some(LidlSource::new(uri));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I ask the price of "([^"]+)"$"#)]
async fn when_ask_price(world: &mut LidlWorld, product: String) {
    let source = world.source.as_ref().expect("source not built");
    world.result = Some(source.price_cents(&product).await.map_err(|e| e.to_string()));
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the price is (\d+\.\d+)$"#)]
fn then_price(world: &mut LidlWorld, expected: String) {
    assert_eq!(
        world.result,
        Some(Ok(Some(cents(&expected)))),
        "price mismatch"
    );
}

#[then("the product is reported as not sold")]
fn then_not_sold(world: &mut LidlWorld) {
    assert_eq!(world.result, Some(Ok(None)), "expected the product to be not sold");
}

#[then("the lookup fails")]
fn then_lookup_fails(world: &mut LidlWorld) {
    assert!(
        matches!(&world.result, Some(Err(_))),
        "expected the lookup to fail, got: {:?}",
        world.result
    );
}

#[then(regex = r#"^the store name is "([^"]+)"$"#)]
fn then_store_name(world: &mut LidlWorld, expected: String) {
    let source = world.source.as_ref().expect("source not built");
    assert_eq!(source.name(), expected, "store name mismatch");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    LidlWorld::run("features/lidl_source.feature").await;
}
