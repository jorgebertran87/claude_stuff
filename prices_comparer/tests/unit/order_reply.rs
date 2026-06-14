use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::basket::{
    BasketSource, FetchError, IdentityNormalizer, PurchasedBasket, PurchasedItem,
};
use prices_comparer::bot::reply_to;
use prices_comparer::comparer::{parse_basket, StoreMatch, StoreSource, Unit, UnitPrice};

// ── Fake store ────────────────────────────────────────────────────────────────

struct FakeStore {
    name: String,
    prices: HashMap<String, UnitPrice>,
}

#[async_trait]
impl StoreSource for FakeStore {
    fn name(&self) -> &str {
        &self.name
    }

    async fn lookup(&self, product: &str, _want: Option<Unit>) -> anyhow::Result<Option<StoreMatch>> {
        Ok(self
            .prices
            .get(product)
            .map(|&price| StoreMatch { name: product.to_string(), price }))
    }
}

// ── Fake basket source ────────────────────────────────────────────────────────

#[derive(Default, PartialEq)]
enum Mode {
    #[default]
    Orders,
    Unavailable,
    NotConfigured,
    Unauthorized,
}

#[derive(Default)]
struct FakeGlovo {
    order: Option<PurchasedBasket>,
    mode: Mode,
}

#[async_trait]
impl BasketSource for FakeGlovo {
    fn name(&self) -> &str {
        "Glovo"
    }

    async fn fetch_basket(
        &self,
        _reference: Option<&str>,
    ) -> Result<Option<PurchasedBasket>, FetchError> {
        match self.mode {
            Mode::Unavailable => Err(FetchError::Unavailable),
            Mode::NotConfigured => Err(FetchError::NotConfigured),
            Mode::Unauthorized => Err(FetchError::Unauthorized),
            Mode::Orders => Ok(self.order.clone()),
        }
    }

    async fn set_token(&self, _token: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World, Default)]
pub struct OrderWorld {
    stores: Vec<Box<dyn StoreSource>>,
    glovo: FakeGlovo,
    reply: Option<String>,
}

impl std::fmt::Debug for OrderWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderWorld").field("reply", &self.reply).finish()
    }
}

fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

fn euros(c: u64) -> String {
    format!("{}.{:02} €", c / 100, c % 100)
}

fn unit(name: &str) -> Unit {
    match name {
        "litre" => Unit::Litre,
        "kilo" => Unit::Kilogram,
        "each" => Unit::Each,
        other => panic!("unit {other:?}"),
    }
}

fn cell(price: &str, unit_name: &str) -> String {
    let label = match unit_name {
        "litre" => "L",
        "kilo" => "kg",
        "each" => "each",
        other => panic!("unit {other:?}"),
    };
    format!("{}/{label}", euros(cents(price)))
}

fn purchased(basket: &str, paid_cents: Option<u64>) -> PurchasedBasket {
    let items = parse_basket(basket)
        .expect("valid basket")
        .into_iter()
        .map(|i| PurchasedItem { name: i.name, quantity: i.quantity, price_cents: None, size: None })
        .collect();
    PurchasedBasket { items, store: None, paid_cents }
}

impl OrderWorld {
    fn reply(&self) -> &str {
        self.reply.as_deref().expect("no reply yet")
    }

    fn assert_contains(&self, needle: &str) {
        assert!(
            self.reply().contains(needle),
            "expected reply to contain {needle:?}, got:\n{}",
            self.reply()
        );
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a store "([^"]+)" pricing "([^"]+)" at (\d+\.\d+) per (\w+)$"#)]
fn given_store(world: &mut OrderWorld, store: String, product: String, price: String, unit_name: String) {
    let up = UnitPrice { cents_per_unit: cents(&price), unit: unit(&unit_name) };
    world.stores.push(Box::new(FakeStore { name: store, prices: HashMap::from([(product, up)]) }));
}

#[given(regex = r#"^a Glovo order of "([^"]+)"$"#)]
fn given_order(world: &mut OrderWorld, basket: String) {
    world.glovo.order = Some(purchased(&basket, None));
}

#[given(regex = r#"^a Glovo order of "([^"]+)" paid (\d+\.\d+)$"#)]
fn given_order_paid(world: &mut OrderWorld, basket: String, paid: String) {
    world.glovo.order = Some(purchased(&basket, Some(cents(&paid))));
}

#[given("an empty Glovo order history")]
fn given_empty(_world: &mut OrderWorld) {}

#[given("a Glovo source that fails to respond")]
fn given_failing(world: &mut OrderWorld) {
    world.glovo.mode = Mode::Unavailable;
}

#[given("Glovo has no token configured")]
fn given_not_configured(world: &mut OrderWorld) {
    world.glovo.mode = Mode::NotConfigured;
}

#[given("the Glovo token has expired")]
fn given_expired(world: &mut OrderWorld) {
    world.glovo.mode = Mode::Unauthorized;
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I message "(.*)"$"#)]
async fn when_message(world: &mut OrderWorld, message: String) {
    let baskets: Vec<Box<dyn BasketSource>> = vec![Box::new(std::mem::take(&mut world.glovo))];
    world.reply = Some(reply_to(&world.stores, &baskets, &IdentityNormalizer, &message).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the reply shows "([^"]+)" at (\d+\.\d+) per (\w+) for "([^"]+)"$"#)]
fn then_priced(world: &mut OrderWorld, product: String, price: String, unit_name: String, store: String) {
    world.assert_contains(&format!("{store}: {product} | {unit_name} | {}", cell(&price, &unit_name)));
}

#[then(regex = r#"^the reply says I paid (\d+\.\d+) on Glovo$"#)]
fn then_paid(world: &mut OrderWorld, paid: String) {
    world.assert_contains(&format!("You paid {} on Glovo", euros(cents(&paid))));
}

#[then("the reply confirms the Glovo token was saved")]
fn then_token_saved(world: &mut OrderWorld) {
    world.assert_contains("Glovo token saved");
}

#[then("the reply says Glovo is not configured")]
fn then_not_configured(world: &mut OrderWorld) {
    world.assert_contains("Glovo is not configured");
}

#[then("the reply says the Glovo token has expired")]
fn then_expired(world: &mut OrderWorld) {
    world.assert_contains("Glovo token has expired");
}

#[then("the reply says Glovo could not be reached")]
fn then_unreachable(world: &mut OrderWorld) {
    world.assert_contains("Glovo could not be reached");
}

#[then("the reply says no Glovo order was found")]
fn then_no_order(world: &mut OrderWorld) {
    world.assert_contains("No Glovo order was found");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    OrderWorld::run("features/order_reply.feature").await;
}
