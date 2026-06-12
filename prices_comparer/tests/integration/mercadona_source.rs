use cucumber::{given, then, when, World};
use prices_comparer::comparer::StoreSource;
use prices_comparer::source::mercadona::MercadonaSource;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct MercadonaWorld {
    // MockServer must be kept alive so the mock remains mounted during the test.
    server: Option<MockServer>,
    source: Option<MercadonaSource>,
    result: Option<Result<Option<u64>, String>>,
}

impl std::fmt::Debug for MercadonaWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MercadonaWorld")
            .field("result", &self.result)
            .finish()
    }
}

impl Default for MercadonaWorld {
    fn default() -> Self {
        Self { server: None, source: None, result: None }
    }
}

/// Parse a price like "1.15" into cents without going through floats.
fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

async fn mount_search(world: &mut MercadonaWorld, term: &str, hits: serde_json::Value) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains(term))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "hits": hits })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock Mercadona API where searching "([^"]+)" finds "([^"]+)" at (\d+\.\d+)$"#)]
async fn given_one_hit(world: &mut MercadonaWorld, term: String, name: String, price: String) {
    // The real product API serves unit_price as a decimal string.
    let hits = json!([
        { "display_name": name, "price_instructions": { "unit_price": price } }
    ]);
    mount_search(world, &term, hits).await;
}

#[given(regex = r#"^a mock Mercadona API where searching "([^"]+)" finds "([^"]+)" at (\d+\.\d+) and "([^"]+)" at (\d+\.\d+)$"#)]
async fn given_two_hits(
    world: &mut MercadonaWorld,
    term: String,
    name_a: String,
    price_a: String,
    name_b: String,
    price_b: String,
) {
    // The Algolia search index serves unit_price as a JSON number.
    let hits = json!([
        { "display_name": name_a, "price_instructions": { "unit_price": price_a.parse::<f64>().unwrap() } },
        { "display_name": name_b, "price_instructions": { "unit_price": price_b.parse::<f64>().unwrap() } }
    ]);
    mount_search(world, &term, hits).await;
}

#[given(regex = r#"^a mock Mercadona API where searching "([^"]+)" finds nothing$"#)]
async fn given_no_hits(world: &mut MercadonaWorld, term: String) {
    mount_search(world, &term, json!([])).await;
}

#[given("a mock Mercadona API that returns HTTP 500")]
async fn given_http_error(world: &mut MercadonaWorld) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Mercadona API that returns invalid JSON")]
async fn given_invalid_json(world: &mut MercadonaWorld) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a Mercadona source pointed at the mock")]
fn given_source(world: &mut MercadonaWorld) {
    let uri = world.server.as_ref().expect("mock server not started").uri();
    world.source = Some(MercadonaSource::new(uri, "test-app".into(), "test-key".into()));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I ask the price of "([^"]+)"$"#)]
async fn when_ask_price(world: &mut MercadonaWorld, product: String) {
    let source = world.source.as_ref().expect("source not built");
    world.result = Some(source.price_cents(&product).await.map_err(|e| e.to_string()));
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the price is (\d+\.\d+)$"#)]
fn then_price(world: &mut MercadonaWorld, expected: String) {
    assert_eq!(
        world.result,
        Some(Ok(Some(cents(&expected)))),
        "price mismatch"
    );
}

#[then("the product is reported as not sold")]
fn then_not_sold(world: &mut MercadonaWorld) {
    assert_eq!(world.result, Some(Ok(None)), "expected the product to be not sold");
}

#[then("the lookup fails")]
fn then_lookup_fails(world: &mut MercadonaWorld) {
    assert!(
        matches!(&world.result, Some(Err(_))),
        "expected the lookup to fail, got: {:?}",
        world.result
    );
}

#[then(regex = r#"^the store name is "([^"]+)"$"#)]
fn then_store_name(world: &mut MercadonaWorld, expected: String) {
    let source = world.source.as_ref().expect("source not built");
    assert_eq!(source.name(), expected, "store name mismatch");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    MercadonaWorld::run("features/mercadona_source.feature").await;
}
