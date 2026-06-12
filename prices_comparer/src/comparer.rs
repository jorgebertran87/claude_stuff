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

/// The full result of comparing a basket across stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// One report per store, in the order the stores were given.
    pub stores: Vec<(String, StoreReport)>,
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
    for store in stores {
        let report = price_basket(store.as_ref(), items).await;
        reports.push((store.name().to_string(), report));
    }

    let missing_everywhere = missing_in_every_responsive_store(items, &reports);
    let cheapest = cheapest_complete_store(&reports);

    Comparison { stores: reports, missing_everywhere, cheapest }
}

async fn price_basket(store: &dyn StoreSource, items: &[BasketItem]) -> StoreReport {
    let mut total_cents = 0u64;
    let mut missing = Vec::new();
    for item in items {
        match store.price_cents(&item.name).await {
            Err(_) => return StoreReport::Unavailable,
            Ok(None) => missing.push(item.name.clone()),
            Ok(Some(cents)) => total_cents += cents * item.quantity,
        }
    }
    if missing.is_empty() {
        StoreReport::Complete { total_cents }
    } else {
        StoreReport::Incomplete { total_cents, missing }
    }
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
