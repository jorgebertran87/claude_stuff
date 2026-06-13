use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::basket::{BasketSource, FetchError, PurchasedBasket};
use prices_comparer::bot::reply_to;
use prices_comparer::comparer::{parse_basket, StoreSource};

// ── Fake store ────────────────────────────────────────────────────────────────

struct FakeStore {
    name: String,
    prices: HashMap<String, u64>,
}

#[async_trait]
impl StoreSource for FakeStore {
    fn name(&self) -> &str {
        &self.name
    }

    async fn price_cents(&self, product: &str) -> anyhow::Result<Option<u64>> {
        Ok(self.prices.get(product).copied())
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
    /// (order id, basket) — newest first; `None` id means "only the latest".
    orders: Vec<(Option<String>, PurchasedBasket)>,
    mode: Mode,
}

#[async_trait]
impl BasketSource for FakeGlovo {
    fn name(&self) -> &str {
        "Glovo"
    }

    async fn fetch_basket(
        &self,
        reference: Option<&str>,
    ) -> Result<Option<PurchasedBasket>, FetchError> {
        match self.mode {
            Mode::Unavailable => return Err(FetchError::Unavailable),
            Mode::NotConfigured => return Err(FetchError::NotConfigured),
            Mode::Unauthorized => return Err(FetchError::Unauthorized),
            Mode::Orders => {}
        }
        Ok(match reference {
            None => self.orders.first().map(|(_, basket)| basket.clone()),
            Some(id) => self
                .orders
                .iter()
                .find(|(order_id, _)| order_id.as_deref() == Some(id))
                .map(|(_, basket)| basket.clone()),
        })
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
        f.debug_struct("OrderWorld")
            .field("reply", &self.reply)
            .finish()
    }
}

/// Parse a price like "1.10" into cents without going through floats.
fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

fn euros(cents: u64) -> String {
    format!("{}.{:02} €", cents / 100, cents % 100)
}

fn purchased(basket: &str, store: Option<String>, paid_cents: Option<u64>) -> PurchasedBasket {
    PurchasedBasket {
        items: parse_basket(basket).expect("valid basket in feature file"),
        store,
        paid_cents,
    }
}

impl OrderWorld {
    fn reply(&self) -> &str {
        self.reply.as_deref().expect("no reply yet")
    }

    fn assert_reply_contains(&self, needle: &str) {
        assert!(
            self.reply().contains(needle),
            "expected reply to contain {needle:?}, got:\n{}",
            self.reply()
        );
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a store "([^"]+)" selling "([^"]+)" at (\d+\.\d+) and "([^"]+)" at (\d+\.\d+)$"#)]
fn given_store_two_products(
    world: &mut OrderWorld,
    store: String,
    product_a: String,
    price_a: String,
    product_b: String,
    price_b: String,
) {
    let prices = HashMap::from([(product_a, cents(&price_a)), (product_b, cents(&price_b))]);
    world.stores.push(Box::new(FakeStore { name: store, prices }));
}

#[given(regex = r#"^a store "([^"]+)" selling "([^"]+)" at (\d+\.\d+)$"#)]
fn given_store_one_product(world: &mut OrderWorld, store: String, product: String, price: String) {
    let prices = HashMap::from([(product, cents(&price))]);
    world.stores.push(Box::new(FakeStore { name: store, prices }));
}

#[given(regex = r#"^a Glovo order from "([^"]+)" of "([^"]+)"$"#)]
fn given_order_from(world: &mut OrderWorld, store: String, basket: String) {
    world.glovo.orders.push((None, purchased(&basket, Some(store), None)));
}

#[given(regex = r#"^a Glovo order from "([^"]+)" of "([^"]+)" paid (\d+\.\d+)$"#)]
fn given_order_from_paid(world: &mut OrderWorld, store: String, basket: String, paid: String) {
    world
        .glovo
        .orders
        .push((None, purchased(&basket, Some(store), Some(cents(&paid)))));
}

#[given(regex = r#"^a Glovo order "([^"]+)" of "([^"]+)"$"#)]
fn given_order_with_id(world: &mut OrderWorld, id: String, basket: String) {
    world.glovo.orders.push((Some(id), purchased(&basket, None, None)));
}

#[given("an empty Glovo order history")]
fn given_empty_history(_world: &mut OrderWorld) {}

#[given("a Glovo source that fails to respond")]
fn given_glovo_failing(world: &mut OrderWorld) {
    world.glovo.mode = Mode::Unavailable;
}

#[given("Glovo has no token configured")]
fn given_glovo_not_configured(world: &mut OrderWorld) {
    world.glovo.mode = Mode::NotConfigured;
}

#[given("the Glovo token has expired")]
fn given_glovo_expired(world: &mut OrderWorld) {
    world.glovo.mode = Mode::Unauthorized;
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I message "(.*)"$"#)]
async fn when_message(world: &mut OrderWorld, message: String) {
    let baskets: Vec<Box<dyn BasketSource>> = vec![Box::new(std::mem::take(&mut world.glovo))];
    world.reply = Some(reply_to(&world.stores, &baskets, &message).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the reply shows "([^"]+)" with total (\d+\.\d+)$"#)]
fn then_total(world: &mut OrderWorld, store: String, total: String) {
    world.assert_reply_contains(&format!("{store}: {}", euros(cents(&total))));
}

#[then(regex = r#"^the reply shows "([^"]+)" as where I bought, with total (\d+\.\d+)$"#)]
fn then_bought_total(world: &mut OrderWorld, store: String, total: String) {
    world.assert_reply_contains(&format!("Bought at {store}: {}", euros(cents(&total))));
}

#[then(regex = r#"^the reply says I could have saved (\d+\.\d+) buying at "([^"]+)"$"#)]
fn then_saved(world: &mut OrderWorld, amount: String, store: String) {
    world.assert_reply_contains(&format!(
        "could have saved {} buying at {store}",
        euros(cents(&amount))
    ));
}

#[then(regex = r#"^the reply says I paid (\d+\.\d+) on Glovo$"#)]
fn then_paid(world: &mut OrderWorld, paid: String) {
    world.assert_reply_contains(&format!("You paid {} on Glovo", euros(cents(&paid))));
}

#[then(regex = r#"^the reply says "([^"]+)" is not a compared store$"#)]
fn then_not_compared(world: &mut OrderWorld, store: String) {
    world.assert_reply_contains(&format!("{store} is not a compared store"));
}

#[then("the reply says no Glovo order was found")]
fn then_no_order(world: &mut OrderWorld) {
    world.assert_reply_contains("No Glovo order was found");
}

#[then("the reply says Glovo could not be reached")]
fn then_unreachable(world: &mut OrderWorld) {
    world.assert_reply_contains("Glovo could not be reached");
}

#[then("the reply confirms the Glovo token was saved")]
fn then_token_saved(world: &mut OrderWorld) {
    world.assert_reply_contains("Glovo token saved");
}

#[then("the reply says Glovo is not configured")]
fn then_not_configured(world: &mut OrderWorld) {
    world.assert_reply_contains("Glovo is not configured");
}

#[then("the reply says the Glovo token has expired")]
fn then_token_expired(world: &mut OrderWorld) {
    world.assert_reply_contains("Glovo token has expired");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    OrderWorld::run("features/order_reply.feature").await;
}
