use async_trait::async_trait;

/// Port: a supermarket that can be asked for the price of a product.
/// Implementations live in the infrastructure layer (one adapter per store).
#[async_trait]
pub trait StoreSource: Send + Sync {
    fn name(&self) -> &str;

    /// Price of one unit in cents. `Ok(None)` means the store does not sell
    /// the product; `Err` means the store could not be reached at all.
    async fn price_cents(&self, product: &str) -> anyhow::Result<Option<u64>>;
}

/// One line of the shopper's basket: a product name and how many units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasketItem {
    pub name: String,
    pub quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    EmptyBasket,
}

impl std::fmt::Display for CompareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompareError::EmptyBasket => write!(f, "the basket is empty"),
        }
    }
}

impl std::error::Error for CompareError {}

/// Outcome of pricing the basket at one store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreReport {
    /// Every product was found; the basket can be bought here for this total.
    Complete { total_cents: u64 },
    /// Some products are not sold here. The partial total covers only the
    /// products that were found; an incomplete store never wins the
    /// cheapest comparison.
    Incomplete { total_cents: u64, missing: Vec<String> },
    /// The store could not be reached; it takes no part in the comparison.
    Unavailable,
}

/// What one basket product costs at each store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPrices {
    pub name: String,
    pub quantity: u64,
    /// `(store name, line price in cents)` per store, in store order.
    /// `None` means the store does not sell it or could not be reached.
    pub per_store: Vec<(String, Option<u64>)>,
}

/// The full result of comparing a basket across stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// One report per store, in the order the stores were given.
    pub stores: Vec<(String, StoreReport)>,
    /// One entry per basket product, with its price at each store.
    pub items: Vec<ItemPrices>,
    /// Products that no responsive store sells.
    pub missing_everywhere: Vec<String>,
    /// The complete store with the lowest total, if any store is complete.
    pub cheapest: Option<String>,
}

/// Parse a basket string like `"milk x3, bread"` into items.
/// A trailing ` x<n>` sets the quantity; it defaults to 1.
pub fn parse_basket(input: &str) -> Result<Vec<BasketItem>, CompareError> {
    let items: Vec<BasketItem> = input
        .split(',')
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(parse_item)
        .collect();
    if items.is_empty() {
        return Err(CompareError::EmptyBasket);
    }
    Ok(items)
}

fn parse_item(raw: &str) -> BasketItem {
    if let Some((name, qty)) = raw.rsplit_once(" x") {
        if let Ok(quantity) = qty.parse::<u64>() {
            if quantity > 0 && !name.trim().is_empty() {
                return BasketItem { name: name.trim().to_string(), quantity };
            }
        }
    }
    BasketItem { name: raw.to_string(), quantity: 1 }
}

/// Price the basket at every store and report totals, missing products and
/// the cheapest complete store.
pub async fn compare(
    stores: &[Box<dyn StoreSource>],
    basket: &str,
) -> Result<Comparison, CompareError> {
    let items = parse_basket(basket)?;
    Ok(compare_items(stores, &items).await)
}

/// Like [`compare`], for a basket that is already parsed (e.g. recovered
/// from an external order rather than typed).
pub async fn compare_items(stores: &[Box<dyn StoreSource>], items: &[BasketItem]) -> Comparison {
    let mut reports = Vec::with_capacity(stores.len());
    // Per store, the line price of each basket item (None = not sold / unreachable).
    let mut grid: Vec<(String, Vec<Option<u64>>)> = Vec::with_capacity(stores.len());
    for store in stores {
        let name = store.name().to_string();
        let (report, line_prices) = price_basket(store.as_ref(), items).await;
        reports.push((name.clone(), report));
        grid.push((name, line_prices));
    }

    // Transpose the grid into a per-item view.
    let item_prices = items
        .iter()
        .enumerate()
        .map(|(i, item)| ItemPrices {
            name: item.name.clone(),
            quantity: item.quantity,
            per_store: grid.iter().map(|(store, prices)| (store.clone(), prices[i])).collect(),
        })
        .collect();

    let missing_everywhere = missing_in_every_responsive_store(items, &reports);
    let cheapest = cheapest_complete_store(&reports);

    Comparison { stores: reports, items: item_prices, missing_everywhere, cheapest }
}

/// Price every item at one store, returning the per-store report and the line
/// price of each item (None = not sold). A store that errors on any item is
/// `Unavailable` with no prices.
async fn price_basket(
    store: &dyn StoreSource,
    items: &[BasketItem],
) -> (StoreReport, Vec<Option<u64>>) {
    let mut line_prices = Vec::with_capacity(items.len());
    for item in items {
        match store.price_cents(&item.name).await {
            Err(_) => return (StoreReport::Unavailable, vec![None; items.len()]),
            Ok(None) => line_prices.push(None),
            Ok(Some(cents)) => line_prices.push(Some(cents * item.quantity)),
        }
    }
    let total_cents = line_prices.iter().flatten().sum();
    let missing: Vec<String> = items
        .iter()
        .zip(&line_prices)
        .filter(|(_, price)| price.is_none())
        .map(|(item, _)| item.name.clone())
        .collect();
    let report = if missing.is_empty() {
        StoreReport::Complete { total_cents }
    } else {
        StoreReport::Incomplete { total_cents, missing }
    };
    (report, line_prices)
}

fn missing_in_every_responsive_store(
    items: &[BasketItem],
    reports: &[(String, StoreReport)],
) -> Vec<String> {
    let responsive: Vec<&StoreReport> = reports
        .iter()
        .map(|(_, report)| report)
        .filter(|report| !matches!(report, StoreReport::Unavailable))
        .collect();
    if responsive.is_empty() {
        return Vec::new();
    }
    items
        .iter()
        .map(|item| &item.name)
        .filter(|name| {
            responsive.iter().all(|report| match report {
                StoreReport::Incomplete { missing, .. } => missing.contains(name),
                _ => false,
            })
        })
        .cloned()
        .collect()
}

fn cheapest_complete_store(reports: &[(String, StoreReport)]) -> Option<String> {
    reports
        .iter()
        .filter_map(|(name, report)| match report {
            StoreReport::Complete { total_cents } => Some((name, total_cents)),
            _ => None,
        })
        .min_by_key(|(_, total)| **total)
        .map(|(name, _)| name.clone())
}
