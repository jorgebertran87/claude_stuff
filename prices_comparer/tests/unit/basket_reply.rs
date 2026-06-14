use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::basket::IdentityNormalizer;
use prices_comparer::bot::reply_to;
use prices_comparer::comparer::{StoreMatch, StoreSource, Unit, UnitPrice};

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

    async fn lookup(&self, product: &str, _description: &str, _want: Option<Unit>) -> anyhow::Result<Option<StoreMatch>> {
        Ok(self
            .prices
            .get(product)
            .map(|&price| StoreMatch { name: product.to_string(), price }))
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
        f.debug_struct("ReplyWorld").field("reply", &self.reply).finish()
    }
}

fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

/// Format a per-unit cell the way the bot renders it, e.g. "0.96 €/L".
fn cell(price: &str, unit_name: &str) -> String {
    let c = cents(price);
    let label = match unit_name {
        "litre" => "L",
        "kilo" => "kg",
        "each" => "each",
        other => panic!("unit {other:?}"),
    };
    format!("{}.{:02} €/{}", c / 100, c % 100, label)
}

fn unit(name: &str) -> Unit {
    match name {
        "litre" => Unit::Litre,
        "kilo" => Unit::Kilogram,
        "each" => Unit::Each,
        other => panic!("unit {other:?}"),
    }
}

impl ReplyWorld {
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
fn given_store_pricing(world: &mut ReplyWorld, store: String, product: String, price: String, unit_name: String) {
    let up = UnitPrice { cents_per_unit: cents(&price), unit: unit(&unit_name) };
    world.stores.push(Box::new(FakeStore { name: store, prices: HashMap::from([(product, up)]) }));
}

#[given(regex = r#"^a store "([^"]+)" that does not sell "([^"]+)"$"#)]
fn given_store_not_selling(world: &mut ReplyWorld, store: String, _product: String) {
    world.stores.push(Box::new(FakeStore { name: store, prices: HashMap::new() }));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I message "(.*)"$"#)]
async fn when_message(world: &mut ReplyWorld, message: String) {
    world.reply = Some(reply_to(&world.stores, &[], &IdentityNormalizer, &message).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the reply shows "([^"]+)" at (\d+\.\d+) per (\w+) for "([^"]+)"$"#)]
fn then_priced(world: &mut ReplyWorld, product: String, price: String, unit_name: String, store: String) {
    world.assert_contains(&format!("{store}: {product} | {unit_name} | {}", cell(&price, &unit_name)));
}

#[then(regex = r#"^the reply marks "([^"]+)" cheapest for "([^"]+)"$"#)]
fn then_cheapest(world: &mut ReplyWorld, store: String, _product: String) {
    let marked = world.reply().lines().any(|l| l.contains(&store) && l.contains("← cheapest"));
    assert!(marked, "expected {store} marked cheapest, got:\n{}", world.reply());
}

#[then(regex = r#"^the reply shows "([^"]+)" with no price$"#)]
fn then_no_price(world: &mut ReplyWorld, store: String) {
    world.assert_contains(&format!("{store}: —"));
}

#[then("the reply explains how to send a basket")]
fn then_usage(world: &mut ReplyWorld) {
    world.assert_contains("Send your basket");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    ReplyWorld::run("features/basket_reply.feature").await;
}
