use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::comparer::{compare, CompareError, Comparison, StoreReport, StoreSource};

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
pub struct ComparerWorld {
    stores: Vec<Box<dyn StoreSource>>,
    result: Option<Result<Comparison, CompareError>>,
}

impl std::fmt::Debug for ComparerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComparerWorld")
            .field("result", &self.result)
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

impl ComparerWorld {
    fn add_store(&mut self, name: String, prices: HashMap<String, u64>, fails: bool) {
        self.stores.push(Box::new(FakeStore { name, prices, fails }));
    }

    fn report_for(&self, store: &str) -> &StoreReport {
        let comparison = match &self.result {
            Some(Ok(comparison)) => comparison,
            other => panic!("expected a successful comparison, got: {other:?}"),
        };
        comparison
            .stores
            .iter()
            .find(|(name, _)| name == store)
            .map(|(_, report)| report)
            .unwrap_or_else(|| panic!("no report for store {store:?}"))
    }

    /// The recorded line price of `product` at `store` (None = not sold).
    fn price_for(&self, product: &str, store: &str) -> Option<u64> {
        let comparison = match &self.result {
            Some(Ok(comparison)) => comparison,
            other => panic!("expected a successful comparison, got: {other:?}"),
        };
        let item = comparison
            .items
            .iter()
            .find(|i| i.name == product)
            .unwrap_or_else(|| panic!("no item {product:?} in comparison"));
        item.per_store
            .iter()
            .find(|(name, _)| name == store)
            .unwrap_or_else(|| panic!("no store {store:?} for item {product:?}"))
            .1
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a store "([^"]+)" selling "([^"]+)" at (\d+\.\d+) and "([^"]+)" at (\d+\.\d+)$"#)]
fn given_store_two_products(
    world: &mut ComparerWorld,
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
fn given_store_one_product(world: &mut ComparerWorld, store: String, product: String, price: String) {
    let prices = HashMap::from([(product, cents(&price))]);
    world.add_store(store, prices, false);
}

#[given(regex = r#"^a store "([^"]+)" that fails to respond$"#)]
fn given_store_failing(world: &mut ComparerWorld, store: String) {
    world.add_store(store, HashMap::new(), true);
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I compare the basket "(.*)"$"#)]
async fn when_compare(world: &mut ComparerWorld, basket: String) {
    world.result = Some(compare(&world.stores, &basket).await);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the total for "([^"]+)" is (\d+\.\d+)$"#)]
fn then_total(world: &mut ComparerWorld, store: String, expected: String) {
    let report = world.report_for(&store);
    assert_eq!(
        report,
        &StoreReport::Complete { total_cents: cents(&expected) },
        "total mismatch for {store}"
    );
}

#[then(regex = r#"^the total for "([^"]+)" is incomplete, missing "([^"]+)"$"#)]
fn then_total_incomplete(world: &mut ComparerWorld, store: String, product: String) {
    let report = world.report_for(&store);
    match report {
        StoreReport::Incomplete { missing, .. } => assert!(
            missing.contains(&product),
            "expected {store} to be missing {product:?}, got: {missing:?}"
        ),
        other => panic!("expected an incomplete total for {store}, got: {other:?}"),
    }
}

#[then(regex = r#"^the store "([^"]+)" is reported as unavailable$"#)]
fn then_unavailable(world: &mut ComparerWorld, store: String) {
    let report = world.report_for(&store);
    assert_eq!(report, &StoreReport::Unavailable, "expected {store} to be unavailable");
}

#[then(regex = r#"^the cheapest store is "([^"]+)"$"#)]
fn then_cheapest(world: &mut ComparerWorld, store: String) {
    match &world.result {
        Some(Ok(comparison)) => assert_eq!(
            comparison.cheapest.as_deref(),
            Some(store.as_str()),
            "cheapest store mismatch"
        ),
        other => panic!("expected a successful comparison, got: {other:?}"),
    }
}

#[then(regex = r#"^"([^"]+)" costs (\d+\.\d+) at "([^"]+)"$"#)]
fn then_item_costs(world: &mut ComparerWorld, product: String, price: String, store: String) {
    assert_eq!(
        world.price_for(&product, &store),
        Some(cents(&price)),
        "expected {product} to cost {price} at {store}"
    );
}

#[then(regex = r#"^"([^"]+)" has no price at "([^"]+)"$"#)]
fn then_item_no_price(world: &mut ComparerWorld, product: String, store: String) {
    assert_eq!(
        world.price_for(&product, &store),
        None,
        "expected {product} to have no price at {store}"
    );
}

#[then(regex = r#"^the product "([^"]+)" is reported as missing in every store$"#)]
fn then_missing_everywhere(world: &mut ComparerWorld, product: String) {
    match &world.result {
        Some(Ok(comparison)) => assert!(
            comparison.missing_everywhere.contains(&product),
            "expected {product:?} in missing_everywhere, got: {:?}",
            comparison.missing_everywhere
        ),
        other => panic!("expected a successful comparison, got: {other:?}"),
    }
}

#[then("the comparison fails with an empty basket error")]
fn then_empty_basket_error(world: &mut ComparerWorld) {
    assert!(
        matches!(&world.result, Some(Err(CompareError::EmptyBasket))),
        "expected EmptyBasket error, got: {:?}",
        world.result
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    ComparerWorld::run("features/price_comparison.feature").await;
}
