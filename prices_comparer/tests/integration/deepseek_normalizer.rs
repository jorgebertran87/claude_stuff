use cucumber::{given, then, when, World};
use prices_comparer::basket::{OrderNormalizer, PurchasedBasket, PurchasedItem};
use prices_comparer::comparer::{parse_size, ItemSize, Unit};
use prices_comparer::normalizer::DeepSeekNormalizer;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct DeepSeekWorld {
    // MockServer must be kept alive so the mock remains mounted during the test.
    server: Option<MockServer>,
    normalizer: Option<DeepSeekNormalizer>,
    order: Vec<PurchasedItem>,
    result: Option<Result<Vec<PurchasedItem>, String>>,
}

impl std::fmt::Debug for DeepSeekWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeepSeekWorld").field("result", &self.result).finish()
    }
}

impl Default for DeepSeekWorld {
    fn default() -> Self {
        Self { server: None, normalizer: None, order: Vec::new(), result: None }
    }
}

/// Parse a price like "1.49" into cents without going through floats.
fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

/// Mount a DeepSeek chat-completions mock whose reply carries `content` as the
/// assistant message.
async fn mount_reply(world: &mut DeepSeekWorld, content: String) {
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

fn item(name: &str) -> PurchasedItem {
    PurchasedItem { name: name.to_string(), quantity: 1, price_cents: None, size: parse_size(name) }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock DeepSeek API that cleans the order to "([^"]+)"$"#)]
async fn given_clean(world: &mut DeepSeekWorld, names: String) {
    let cleaned: Vec<Value> = names.split(", ").map(|n| json!({ "name": n })).collect();
    mount_reply(world, json!(cleaned).to_string()).await;
}

#[given(regex = r#"^a mock DeepSeek API that cleans the order to "([^"]+)" keeping quantity (\d+) and price (\d+\.\d+)$"#)]
async fn given_clean_keeping(world: &mut DeepSeekWorld, name: String, quantity: u64, price: String) {
    let cleaned = json!([{ "name": name, "quantity": quantity, "price_cents": cents(&price) }]);
    mount_reply(world, cleaned.to_string()).await;
}

#[given("a mock DeepSeek API that returns HTTP 500")]
async fn given_http_error(world: &mut DeepSeekWorld) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;
    world.server = Some(server);
}

#[given(regex = r#"^a mock DeepSeek API that replies "([^"]+)"$"#)]
async fn given_no_array(world: &mut DeepSeekWorld, reply: String) {
    mount_reply(world, reply).await;
}

#[given("a DeepSeek normalizer pointed at the mock")]
fn given_normalizer(world: &mut DeepSeekWorld) {
    let uri = world.server.as_ref().expect("mock server not started").uri();
    world.normalizer =
        Some(DeepSeekNormalizer::with_base_url(uri, "test-key".into(), "deepseek-chat".into()));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I normalize an order of "([^"]+)" and "([^"]+)"$"#)]
async fn when_normalize_two(world: &mut DeepSeekWorld, first: String, second: String) {
    world.order = vec![item(&first), item(&second)];
    run(world).await;
}

#[when(regex = r#"^I normalize an order of "([^"]+)" with quantity (\d+) priced (\d+\.\d+)$"#)]
async fn when_normalize_detailed(world: &mut DeepSeekWorld, name: String, quantity: u64, price: String) {
    world.order = vec![PurchasedItem {
        name: name.clone(),
        quantity,
        price_cents: Some(cents(&price)),
        size: parse_size(&name),
    }];
    run(world).await;
}

#[when(regex = r#"^I normalize an order of "([^"]+)"$"#)]
async fn when_normalize_one(world: &mut DeepSeekWorld, name: String) {
    world.order = vec![item(&name)];
    run(world).await;
}

async fn run(world: &mut DeepSeekWorld) {
    let normalizer = world.normalizer.as_ref().expect("normalizer not built");
    let basket = PurchasedBasket { items: world.order.clone(), store: None, paid_cents: None };
    world.result = Some(normalizer.normalize(&basket).await.map_err(|e| e.to_string()));
}

// ── Then ──────────────────────────────────────────────────────────────────────

impl DeepSeekWorld {
    fn cleaned(&self) -> &[PurchasedItem] {
        match &self.result {
            Some(Ok(items)) => items,
            other => panic!("expected a successful normalization, got: {other:?}"),
        }
    }
}

#[then(regex = r#"^the cleaned names are "([^"]+)" and "([^"]+)"$"#)]
fn then_names(world: &mut DeepSeekWorld, first: String, second: String) {
    let names: Vec<&str> = world.cleaned().iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec![first.as_str(), second.as_str()], "cleaned names mismatch");
}

#[then(regex = r#"^the cleaned line "([^"]+)" keeps quantity (\d+), price (\d+\.\d+) and size (\d+) (litre|kilo)$"#)]
fn then_keeps(world: &mut DeepSeekWorld, name: String, quantity: u64, price: String, amount: f64, unit_name: String) {
    let unit = match unit_name.as_str() {
        "litre" => Unit::Litre,
        "kilo" => Unit::Kilogram,
        other => panic!("unknown unit {other:?}"),
    };
    let line = world
        .cleaned()
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no cleaned line {name:?}"));
    assert_eq!(line.quantity, quantity, "quantity mismatch");
    assert_eq!(line.price_cents, Some(cents(&price)), "price mismatch");
    assert_eq!(line.size, Some(ItemSize { amount, unit }), "size mismatch");
}

#[then("the normalization fails")]
fn then_fails(world: &mut DeepSeekWorld) {
    assert!(
        matches!(&world.result, Some(Err(_))),
        "expected normalization to fail, got: {:?}",
        world.result
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    DeepSeekWorld::run("features/deepseek_normalizer.feature").await;
}
