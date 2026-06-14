use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::basket::{
    BasketSource, FetchError, OrderNormalizer, PurchasedBasket, PurchasedItem,
};
use prices_comparer::bot::reply_to;
use prices_comparer::comparer::{parse_size, StoreMatch, StoreSource, Unit, UnitPrice};

// ── Fakes ──────────────────────────────────────────────────────────────────────

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
                size: i.size,
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

fn single_order(item: &str, price_cents: Option<u64>) -> PurchasedBasket {
    PurchasedBasket {
        items: vec![PurchasedItem {
            name: item.to_string(),
            quantity: 1,
            price_cents,
            size: parse_size(item),
        }],
        store: None,
        paid_cents: None,
    }
}

impl NormWorld {
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
fn given_store(world: &mut NormWorld, store: String, product: String, price: String, unit_name: String) {
    let up = UnitPrice { cents_per_unit: cents(&price), unit: unit(&unit_name) };
    world.stores.push(Box::new(FakeStore { name: store, prices: HashMap::from([(product, up)]) }));
}

#[given(regex = r#"^a Glovo order of "(.+)" priced (\d+\.\d+)$"#)]
fn given_order_priced(world: &mut NormWorld, item: String, price: String) {
    world.order = Some(single_order(&item, Some(cents(&price))));
}

#[given(regex = r#"^a Glovo order of "(.+)"$"#)]
fn given_order(world: &mut NormWorld, item: String) {
    world.order = Some(single_order(&item, None));
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

#[then(regex = r#"^the reply shows "(.+)" at (\d+\.\d+) per (\w+) for "([^"]+)"$"#)]
fn then_priced(world: &mut NormWorld, product: String, price: String, unit_name: String, store: String) {
    world.assert_contains(&format!("{store}: {product} | {unit_name} | {}", cell(&price, &unit_name)));
}

#[then(regex = r#"^the reply shows the Glovo price (\d+\.\d+) per (\w+)$"#)]
fn then_glovo_price(world: &mut NormWorld, price: String, unit_name: String) {
    let cell = cell(&price, &unit_name);
    let row = store_row(world.reply(), "Glovo");
    assert!(row.contains(&cell), "expected Glovo row to show {cell:?}, got: {row:?}");
}

#[then(regex = r#"^the reply marks "([^"]+)" as the cheapest$"#)]
fn then_marks_cheapest(world: &mut NormWorld, store: String) {
    let row = store_row(world.reply(), &store);
    assert!(row.contains("← cheapest"), "expected {store:?} marked cheapest, got: {row:?}");
}

/// The reply row for a source, e.g. the line starting "Mercadona: ".
fn store_row<'a>(reply: &'a str, store: &str) -> &'a str {
    let prefix = format!("{store}: ");
    reply
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no row for {store:?} in:\n{reply}"))
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    NormWorld::run("features/order_normalization.feature").await;
}
