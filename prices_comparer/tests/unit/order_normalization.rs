use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::basket::{BasketSource, FetchError, OrderNormalizer, PurchasedBasket, PurchasedItem};
use prices_comparer::bot::reply_to;
use prices_comparer::comparer::StoreSource;

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

// ── Fake Glovo basket source ────────────────────────────────────────────────

struct FakeGlovo {
    order: Option<PurchasedBasket>,
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
        Ok(self.order.clone())
    }
}

// ── Fake normalizer ──────────────────────────────────────────────────────────

struct FakeNormalizer {
    clean: HashMap<String, String>,
    fails: bool,
}

#[async_trait]
impl OrderNormalizer for FakeNormalizer {
    async fn normalize(&self, basket: &PurchasedBasket) -> anyhow::Result<Vec<PurchasedItem>> {
        if self.fails {
            anyhow::bail!("normalizer unavailable");
        }
        Ok(basket
            .items
            .iter()
            .map(|i| PurchasedItem {
                name: self.clean.get(&i.name).cloned().unwrap_or_else(|| i.name.clone()),
                quantity: i.quantity,
                price_cents: i.price_cents,
            })
            .collect())
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World, Default)]
pub struct NormWorld {
    stores: Vec<Box<dyn StoreSource>>,
    order: Option<PurchasedBasket>,
    clean: HashMap<String, String>,
    normalizer_fails: bool,
    reply: Option<String>,
}

impl std::fmt::Debug for NormWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NormWorld").field("reply", &self.reply).finish()
    }
}

fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

fn euros(cents: u64) -> String {
    format!("{}.{:02} €", cents / 100, cents % 100)
}

/// Build a single-item Glovo order; the item name may contain commas, so the
/// whole string (minus an optional trailing " xN") is one product.
fn single_order(store: &str, item: &str, price_cents: Option<u64>) -> PurchasedBasket {
    let (name, quantity) = match item.rsplit_once(" x") {
        Some((n, q)) if q.parse::<u64>().is_ok() => (n.to_string(), q.parse().unwrap()),
        _ => (item.to_string(), 1),
    };
    PurchasedBasket {
        items: vec![PurchasedItem { name, quantity, price_cents }],
        store: Some(store.to_string()),
        paid_cents: None,
    }
}

impl NormWorld {
    fn reply(&self) -> &str {
        self.reply.as_deref().expect("no reply yet")
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a store "([^"]+)" selling "([^"]+)" at (\d+\.\d+)$"#)]
fn given_store(world: &mut NormWorld, store: String, product: String, price: String) {
    let prices = HashMap::from([(product, cents(&price))]);
    world.stores.push(Box::new(FakeStore { name: store, prices }));
}

#[given(regex = r#"^a Glovo order from "([^"]+)" of "(.+)" priced (\d+\.\d+)$"#)]
fn given_order_priced(world: &mut NormWorld, store: String, item: String, price: String) {
    world.order = Some(single_order(&store, &item, Some(cents(&price))));
}

#[given(regex = r#"^a Glovo order from "([^"]+)" of "(.+)"$"#)]
fn given_order(world: &mut NormWorld, store: String, item: String) {
    world.order = Some(single_order(&store, &item, None));
}

#[given(regex = r#"^the normalizer cleans "(.+)" to "(.+)"$"#)]
fn given_clean(world: &mut NormWorld, from: String, to: String) {
    world.clean.insert(from, to);
}

#[given("the normalizer is unavailable")]
fn given_unavailable(world: &mut NormWorld) {
    world.normalizer_fails = true;
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I message "(.*)"$"#)]
async fn when_message(world: &mut NormWorld, message: String) {
    let baskets: Vec<Box<dyn BasketSource>> =
        vec![Box::new(FakeGlovo { order: world.order.clone() })];
    let normalizer = FakeNormalizer { clean: world.clean.clone(), fails: world.normalizer_fails };
    world.reply = Some(reply_to(&world.stores, &baskets, &normalizer, &message).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the reply shows "([^"]+)" with total (\d+\.\d+)$"#)]
fn then_total(world: &mut NormWorld, store: String, total: String) {
    let needle = format!("{store}: {}", euros(cents(&total)));
    assert!(
        world.reply().contains(&needle),
        "expected reply to contain {needle:?}, got:\n{}",
        world.reply()
    );
}

#[then(regex = r#"^the reply lists "([^"]+)" priced (\d+\.\d+)$"#)]
fn then_lists_priced(world: &mut NormWorld, name: String, price: String) {
    let reply = world.reply();
    let price = euros(cents(&price));
    assert!(
        reply.contains(&name) && reply.contains(&price),
        "expected reply to list {name:?} priced {price:?}, got:\n{reply}"
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    NormWorld::run("features/order_normalization.feature").await;
}
