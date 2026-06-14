use cucumber::{given, then, when, World};
use prices_comparer::comparer::{StoreMatch, StoreSource, Unit, UnitPrice};
use prices_comparer::source::dia::DiaSource;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct DiaWorld {
    // MockServer must be kept alive so the mock remains mounted during the test.
    server: Option<MockServer>,
    source: Option<DiaSource>,
    result: Option<Result<Option<StoreMatch>, String>>,
}

impl std::fmt::Debug for DiaWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiaWorld")
            .field("result", &self.result)
            .finish()
    }
}

impl Default for DiaWorld {
    fn default() -> Self {
        Self { server: None, source: None, result: None }
    }
}

fn unit(name: &str) -> Unit {
    match name {
        "litres" | "litre" => Unit::Litre,
        "kilos" | "kilo" => Unit::Kilogram,
        "pieces" | "piece" => Unit::Each,
        other => panic!("unknown measure {other:?}"),
    }
}

/// Parse a price like "1.05" into cents without going through floats.
fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

/// Mount a FlareSolverr mock whose solution renders the given page content.
/// `term` must appear in the request body (FlareSolverr receives the Dia
/// search URL, which carries the query).
async fn mount_solution(world: &mut DiaWorld, term: &str, page: String) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1"))
        .and(body_string_contains(term))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "solution": { "response": page }
            })),
        )
        .mount(&server)
        .await;
    world.server = Some(server);
}

/// Wrap search items the way a browser renders a JSON endpoint: inside <pre>.
fn rendered_search_page(items: serde_json::Value) -> String {
    let body = json!({ "search_items": items });
    format!("<html><head></head><body><pre>{body}</pre></body></html>")
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock FlareSolverr where searching Dia for "([^"]+)" finds "([^"]+)" at (\d+\.\d+)$"#)]
async fn given_one_hit(world: &mut DiaWorld, term: String, name: String, price: String) {
    let items = json!([
        { "display_name": name, "prices": { "price": price.parse::<f64>().unwrap() } }
    ]);
    mount_solution(world, &term, rendered_search_page(items)).await;
}

#[given(regex = r#"^a mock FlareSolverr where searching Dia for "([^"]+)" finds "([^"]+)" at (\d+\.\d+) and "([^"]+)" at (\d+\.\d+)$"#)]
async fn given_two_hits(
    world: &mut DiaWorld,
    term: String,
    name_a: String,
    price_a: String,
    name_b: String,
    price_b: String,
) {
    let items = json!([
        { "display_name": name_a, "prices": { "price": price_a.parse::<f64>().unwrap() } },
        { "display_name": name_b, "prices": { "price": price_b.parse::<f64>().unwrap() } }
    ]);
    mount_solution(world, &term, rendered_search_page(items)).await;
}

#[given(regex = r#"^a mock FlareSolverr where searching Dia for "([^"]+)" finds nothing$"#)]
async fn given_no_hits(world: &mut DiaWorld, term: String) {
    mount_solution(world, &term, rendered_search_page(json!([]))).await;
}

#[given("a mock FlareSolverr that returns HTTP 500")]
async fn given_http_error(world: &mut DiaWorld) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given("a mock FlareSolverr where searching Dia returns a page with no product data")]
async fn given_no_product_data(world: &mut DiaWorld) {
    let page = "<html><body><h1>Un momento...</h1></body></html>".to_string();
    mount_solution(world, "dia.es", page).await;
}

#[given("a Dia source pointed at the mock")]
fn given_source(world: &mut DiaWorld) {
    let uri = world.server.as_ref().expect("mock server not started").uri();
    world.source = Some(DiaSource::new(uri, None));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I ask the price of "([^"]+)"$"#)]
async fn when_ask_price(world: &mut DiaWorld, product: String) {
    let source = world.source.as_ref().expect("source not built");
    world.result = Some(source.lookup(&product, &product, None).await.map_err(|e| e.to_string()));
}

#[when(regex = r#"^I ask the price of "([^"]+)" measured in (\w+)$"#)]
async fn when_ask_price_measured(world: &mut DiaWorld, product: String, measure: String) {
    let source = world.source.as_ref().expect("source not built");
    world.result =
        Some(source.lookup(&product, &product, Some(unit(&measure))).await.map_err(|e| e.to_string()));
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the per-unit price is (\d+\.\d+) per litre$"#)]
fn then_price(world: &mut DiaWorld, expected: String) {
    let matched = match &world.result {
        Some(Ok(Some(m))) => m,
        other => panic!("expected a matched price, got: {other:?}"),
    };
    assert_eq!(
        matched.price,
        UnitPrice { cents_per_unit: cents(&expected), unit: Unit::Litre },
        "per-unit price mismatch"
    );
}

#[then("the product is reported as not sold")]
fn then_not_sold(world: &mut DiaWorld) {
    assert_eq!(world.result, Some(Ok(None)), "expected the product to be not sold");
}

#[then("the lookup fails")]
fn then_lookup_fails(world: &mut DiaWorld) {
    assert!(
        matches!(&world.result, Some(Err(_))),
        "expected the lookup to fail, got: {:?}",
        world.result
    );
}

#[then(regex = r#"^the store name is "([^"]+)"$"#)]
fn then_store_name(world: &mut DiaWorld, expected: String) {
    let source = world.source.as_ref().expect("source not built");
    assert_eq!(source.name(), expected, "store name mismatch");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    DiaWorld::run("features/dia_source.feature").await;
}
