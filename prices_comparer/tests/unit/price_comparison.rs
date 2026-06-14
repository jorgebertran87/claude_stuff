use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::comparer::{
    compare, CompareError, Comparison, StoreMatch, StoreSource, Unit, UnitPrice,
};

// ── Fake store ────────────────────────────────────────────────────────────────

struct FakeStore {
    name: String,
    prices: HashMap<String, UnitPrice>,
    fails: bool,
}

#[async_trait]
impl StoreSource for FakeStore {
    fn name(&self) -> &str {
        &self.name
    }

    async fn lookup(&self, product: &str, _description: &str, _want: Option<Unit>) -> anyhow::Result<Option<StoreMatch>> {
        if self.fails {
            anyhow::bail!("store unreachable");
        }
        Ok(self
            .prices
            .get(product)
            .map(|&price| StoreMatch { name: product.to_string(), price }))
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World, Default)]
pub struct ComparerWorld {
    stores: Vec<Box<dyn StoreSource>>,
    result: Option<Result<Comparison, CompareError>>,
}

impl std::fmt::Debug for ComparerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComparerWorld").field("result", &self.result).finish()
    }
}

fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

fn unit(name: &str) -> Unit {
    match name {
        "litre" | "litres" | "l" => Unit::Litre,
        "kilo" | "kilogram" | "kg" => Unit::Kilogram,
        "each" | "unit" => Unit::Each,
        other => panic!("unknown unit {other:?}"),
    }
}

impl ComparerWorld {
    fn comparison(&self) -> &Comparison {
        match &self.result {
            Some(Ok(c)) => c,
            other => panic!("expected a successful comparison, got: {other:?}"),
        }
    }

    fn price_for(&self, product: &str, store: &str) -> Option<UnitPrice> {
        let item = self
            .comparison()
            .items
            .iter()
            .find(|i| i.name == product)
            .unwrap_or_else(|| panic!("no item {product:?}"));
        item.per_store
            .iter()
            .find(|(name, _)| name == store)
            .unwrap_or_else(|| panic!("no store {store:?} for {product:?}"))
            .1
            .as_ref()
            .map(|m| m.price)
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a store "([^"]+)" pricing "([^"]+)" at (\d+\.\d+) per (\w+)$"#)]
fn given_store_pricing(
    world: &mut ComparerWorld,
    store: String,
    product: String,
    price: String,
    unit_name: String,
) {
    let up = UnitPrice { cents_per_unit: cents(&price), unit: unit(&unit_name) };
    world.stores.push(Box::new(FakeStore {
        name: store,
        prices: HashMap::from([(product, up)]),
        fails: false,
    }));
}

#[given(regex = r#"^a store "([^"]+)" that does not sell "([^"]+)"$"#)]
fn given_store_not_selling(world: &mut ComparerWorld, store: String, _product: String) {
    world.stores.push(Box::new(FakeStore {
        name: store,
        prices: HashMap::new(),
        fails: false,
    }));
}

#[given(regex = r#"^a store "([^"]+)" that fails to respond$"#)]
fn given_store_failing(world: &mut ComparerWorld, store: String) {
    world.stores.push(Box::new(FakeStore {
        name: store,
        prices: HashMap::new(),
        fails: true,
    }));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I compare the basket "(.*)"$"#)]
async fn when_compare(world: &mut ComparerWorld, basket: String) {
    world.result = Some(compare(&world.stores, &basket).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^"([^"]+)" costs (\d+\.\d+) per (\w+) at "([^"]+)"$"#)]
fn then_costs(world: &mut ComparerWorld, product: String, price: String, unit_name: String, store: String) {
    assert_eq!(
        world.price_for(&product, &store),
        Some(UnitPrice { cents_per_unit: cents(&price), unit: unit(&unit_name) }),
        "per-unit price mismatch for {product} at {store}"
    );
}

#[then(regex = r#"^"([^"]+)" has no price at "([^"]+)"$"#)]
fn then_no_price(world: &mut ComparerWorld, product: String, store: String) {
    assert_eq!(world.price_for(&product, &store), None, "expected no price for {product} at {store}");
}

#[then(regex = r#"^the cheapest store for "([^"]+)" is "([^"]+)"$"#)]
fn then_cheapest(world: &mut ComparerWorld, product: String, store: String) {
    let item = world
        .comparison()
        .items
        .iter()
        .find(|i| i.name == product)
        .unwrap_or_else(|| panic!("no item {product:?}"));
    assert_eq!(item.cheapest.as_deref(), Some(store.as_str()), "cheapest store mismatch");
}

#[then("the comparison fails with an empty basket error")]
fn then_empty(world: &mut ComparerWorld) {
    assert!(
        matches!(&world.result, Some(Err(CompareError::EmptyBasket))),
        "expected EmptyBasket, got: {:?}",
        world.result
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    ComparerWorld::run("features/price_comparison.feature").await;
}
