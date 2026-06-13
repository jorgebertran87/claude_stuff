use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::basket::IdentityNormalizer;
use prices_comparer::bot::reply_to;
use prices_comparer::comparer::StoreSource;

// ── Fake store ────────────────────────────────────────────────────────────────

struct FakeStore {
    name: String,
    prices: HashMap<String, u64>,
    fails: bool,
}

#[async_trait]
impl StoreSource for FakeStore {
    fn name(&self) -> &str {
        &self.name
    }

    async fn price_cents(&self, product: &str) -> anyhow::Result<Option<u64>> {
        if self.fails {
            anyhow::bail!("store unreachable");
        }
        Ok(self.prices.get(product).copied())
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World, Default)]
pub struct ReplyWorld {
    stores: Vec<Box<dyn StoreSource>>,
    reply: Option<String>,
}

impl std::fmt::Debug for ReplyWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplyWorld")
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

impl ReplyWorld {
    fn add_store(&mut self, name: String, prices: HashMap<String, u64>, fails: bool) {
        self.stores.push(Box::new(FakeStore { name, prices, fails }));
    }

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
    world: &mut ReplyWorld,
    store: String,
    product_a: String,
    price_a: String,
    product_b: String,
    price_b: String,
) {
    let prices = HashMap::from([(product_a, cents(&price_a)), (product_b, cents(&price_b))]);
    world.add_store(store, prices, false);
}

#[given(regex = r#"^a store "([^"]+)" selling "([^"]+)" at (\d+\.\d+)$"#)]
fn given_store_one_product(world: &mut ReplyWorld, store: String, product: String, price: String) {
    let prices = HashMap::from([(product, cents(&price))]);
    world.add_store(store, prices, false);
}

#[given(regex = r#"^a store "([^"]+)" that fails to respond$"#)]
fn given_store_failing(world: &mut ReplyWorld, store: String) {
    world.add_store(store, HashMap::new(), true);
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I message "(.*)"$"#)]
async fn when_message(world: &mut ReplyWorld, message: String) {
    world.reply = Some(reply_to(&world.stores, &[], &IdentityNormalizer, &message).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the reply shows "([^"]+)" with total (\d+\.\d+)$"#)]
fn then_total(world: &mut ReplyWorld, store: String, total: String) {
    world.assert_reply_contains(&format!("{store}: {}", euros(cents(&total))));
}

#[then(regex = r#"^the reply marks "([^"]+)" as the cheapest$"#)]
fn then_cheapest(world: &mut ReplyWorld, store: String) {
    let marked = world
        .reply()
        .lines()
        .any(|line| line.starts_with(&format!("{store}:")) && line.contains("← cheapest"));
    assert!(
        marked,
        "expected {store} marked as cheapest, got:\n{}",
        world.reply()
    );
}

#[then(regex = r#"^the reply shows "([^"]+)" as where I bought, with total (\d+\.\d+)$"#)]
fn then_bought_total(world: &mut ReplyWorld, store: String, total: String) {
    world.assert_reply_contains(&format!("Bought at {store}: {}", euros(cents(&total))));
}

#[then(regex = r#"^the reply says I could have saved (\d+\.\d+) buying at "([^"]+)"$"#)]
fn then_saved(world: &mut ReplyWorld, amount: String, store: String) {
    world.assert_reply_contains(&format!(
        "could have saved {} buying at {store}",
        euros(cents(&amount))
    ));
}

#[then("the reply says I bought at the cheapest store")]
fn then_bought_cheapest(world: &mut ReplyWorld) {
    world.assert_reply_contains("bought at the cheapest store");
}

#[then(regex = r#"^the reply shows "([^"]+)" as incomplete, missing "([^"]+)"$"#)]
fn then_incomplete(world: &mut ReplyWorld, store: String, product: String) {
    world.assert_reply_contains(&format!("{store}: incomplete (missing: {product})"));
}

#[then(regex = r#"^the reply shows "([^"]+)" as unavailable$"#)]
fn then_unavailable(world: &mut ReplyWorld, store: String) {
    world.assert_reply_contains(&format!("{store}: unavailable"));
}

#[then("the reply says the bought total could not be compared")]
fn then_not_comparable(world: &mut ReplyWorld) {
    world.assert_reply_contains("could not be compared");
}

#[then(regex = r#"^the reply says "([^"]+)" is not a known store$"#)]
fn then_unknown_store(world: &mut ReplyWorld, store: String) {
    world.assert_reply_contains(&format!("{store} is not a known store"));
}

#[then(regex = r#"^the reply lists "([^"]+)" and "([^"]+)" as the known stores$"#)]
fn then_known_stores(world: &mut ReplyWorld, store_a: String, store_b: String) {
    world.assert_reply_contains(&format!("Known stores: {store_a}, {store_b}"));
}

#[then("the reply explains how to send a basket")]
fn then_usage(world: &mut ReplyWorld) {
    world.assert_reply_contains("Send your basket");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    ReplyWorld::run("features/basket_reply.feature").await;
}
