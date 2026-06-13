use cucumber::{given, then, when, World};
use prices_comparer::basket::{BasketSource, FetchError, PurchasedBasket};
use prices_comparer::source::glovo::GlovoSource;
use prices_comparer::token_store::TokenStore;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct GlovoWorld {
    // MockServer and TempDir must be kept alive for the duration of the test.
    server: Option<MockServer>,
    dir: tempfile::TempDir,
    source: Option<GlovoSource>,
    result: Option<Result<Option<PurchasedBasket>, FetchError>>,
}

impl std::fmt::Debug for GlovoWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlovoWorld")
            .field("result", &self.result)
            .finish()
    }
}

impl Default for GlovoWorld {
    fn default() -> Self {
        Self {
            server: None,
            dir: tempfile::tempdir().unwrap(),
            source: None,
            result: None,
        }
    }
}

impl GlovoWorld {
    /// Build a source against the mock, optionally seeding a token.
    fn build_source(&mut self, token: Option<&str>) {
        let uri = self.server.as_ref().expect("mock server not started").uri();
        let tokens = TokenStore::new(self.dir.path().join("glovo_token"));
        if let Some(token) = token {
            tokens.set(token).unwrap();
        }
        self.source = Some(GlovoSource::new(uri, tokens));
    }
}

/// Parse a price like "3.50" into cents without going through floats.
fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

/// Turn "milk x2, bread" into the products array of a Glovo order.
fn products(basket: &str) -> serde_json::Value {
    let items: Vec<serde_json::Value> = basket
        .split(',')
        .map(str::trim)
        .map(|raw| match raw.rsplit_once(" x") {
            Some((name, qty)) if qty.parse::<u64>().is_ok() => {
                json!({ "name": name, "quantity": qty.parse::<u64>().unwrap() })
            }
            _ => json!({ "name": raw, "quantity": 1 }),
        })
        .collect();
    json!(items)
}

fn order(id: u64, store: &str, basket: &str, paid: &str) -> serde_json::Value {
    json!({
        "id": id,
        "store_name": store,
        "paid": paid.parse::<f64>().unwrap(),
        "products": products(basket)
    })
}

async fn mount_orders(world: &mut GlovoWorld, orders: serde_json::Value) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v3/customer/orders"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "orders": orders })))
        .mount(&server)
        .await;
    world.server = Some(server);
}

impl GlovoWorld {
    fn basket(&self) -> &PurchasedBasket {
        match &self.result {
            Some(Ok(Some(basket))) => basket,
            other => panic!("expected a fetched basket, got: {other:?}"),
        }
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock Glovo API with an order from "([^"]+)" of "([^"]+)" paid (\d+\.\d+)$"#)]
async fn given_one_order(world: &mut GlovoWorld, store: String, basket: String, paid: String) {
    mount_orders(world, json!([order(1, &store, &basket, &paid)])).await;
}

#[given(regex = r#"^a mock Glovo API with order "([^"]+)" from "([^"]+)" of "([^"]+)" paid (\d+\.\d+) and order "([^"]+)" from "([^"]+)" of "([^"]+)" paid (\d+\.\d+)$"#)]
#[allow(clippy::too_many_arguments)]
async fn given_two_orders(
    world: &mut GlovoWorld,
    id_a: String,
    store_a: String,
    basket_a: String,
    paid_a: String,
    id_b: String,
    store_b: String,
    basket_b: String,
    paid_b: String,
) {
    mount_orders(
        world,
        json!([
            order(id_a.parse().unwrap(), &store_a, &basket_a, &paid_a),
            order(id_b.parse().unwrap(), &store_b, &basket_b, &paid_b)
        ]),
    )
    .await;
}

#[given("a mock Glovo API with no orders")]
async fn given_no_orders(world: &mut GlovoWorld) {
    mount_orders(world, json!([])).await;
}

#[given("a mock Glovo API that returns HTTP 500")]
async fn given_http_error(world: &mut GlovoWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Glovo API that returns invalid JSON")]
async fn given_invalid_json(world: &mut GlovoWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock Glovo API that rejects the token as unauthorized")]
async fn given_unauthorized(world: &mut GlovoWorld) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a Glovo source pointed at the mock")]
fn given_source(world: &mut GlovoWorld) {
    world.build_source(Some("test-token"));
}

#[given("a Glovo source with no token")]
fn given_source_no_token(world: &mut GlovoWorld) {
    world.build_source(None);
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when("I fetch the last order")]
async fn when_fetch_last(world: &mut GlovoWorld) {
    let source = world.source.as_ref().expect("source not built");
    world.result = Some(source.fetch_basket(None).await);
}

#[when(regex = r#"^I fetch order "([^"]+)"$"#)]
async fn when_fetch_by_id(world: &mut GlovoWorld, id: String) {
    let source = world.source.as_ref().expect("source not built");
    world.result = Some(source.fetch_basket(Some(&id)).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the basket has "([^"]+)" with quantity (\d+)$"#)]
fn then_item(world: &mut GlovoWorld, name: String, quantity: u64) {
    let items = &world.basket().items;
    assert!(
        items.iter().any(|i| i.name == name && i.quantity == quantity),
        "expected item {name:?} x{quantity}, got: {items:?}"
    );
}

#[then(regex = r#"^the basket was bought at "([^"]+)"$"#)]
fn then_bought_at(world: &mut GlovoWorld, store: String) {
    assert_eq!(world.basket().store.as_deref(), Some(store.as_str()), "store mismatch");
}

#[then(regex = r#"^the basket was paid (\d+\.\d+)$"#)]
fn then_paid(world: &mut GlovoWorld, paid: String) {
    assert_eq!(world.basket().paid_cents, Some(cents(&paid)), "paid total mismatch");
}

#[then("no order is found")]
fn then_no_order(world: &mut GlovoWorld) {
    assert!(
        matches!(&world.result, Some(Ok(None))),
        "expected no order, got: {:?}",
        world.result
    );
}

#[then("the fetch reports Glovo is unavailable")]
fn then_unavailable(world: &mut GlovoWorld) {
    assert_eq!(world.result, Some(Err(FetchError::Unavailable)), "expected unavailable");
}

#[then("the fetch reports the token is not configured")]
fn then_not_configured(world: &mut GlovoWorld) {
    assert_eq!(world.result, Some(Err(FetchError::NotConfigured)), "expected not configured");
}

#[then("the fetch reports the token has expired")]
fn then_expired(world: &mut GlovoWorld) {
    assert_eq!(world.result, Some(Err(FetchError::Unauthorized)), "expected unauthorized");
}

#[then(regex = r#"^the basket source name is "([^"]+)"$"#)]
fn then_source_name(world: &mut GlovoWorld, expected: String) {
    let source = world.source.as_ref().expect("source not built");
    assert_eq!(source.name(), expected, "source name mismatch");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    GlovoWorld::run("features/glovo_source.feature").await;
}
