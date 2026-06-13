use async_trait::async_trait;

/// A measurement unit prices are normalized to for comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Litre,
    Kilogram,
    Each,
}

impl Unit {
    /// Short label for display: "L", "kg", "each".
    pub fn label(&self) -> &'static str {
        match self {
            Unit::Litre => "L",
            Unit::Kilogram => "kg",
            Unit::Each => "each",
        }
    }
}

/// A price normalized to one standard unit (e.g. 0.96 €/L).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitPrice {
    pub cents_per_unit: u64,
    pub unit: Unit,
}

/// A product amount in a canonical unit (litres, kilograms, or pieces),
/// parsed from a product name like "Leche Entera, 1L" or "Queso 250g".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemSize {
    pub amount: f64,
    pub unit: Unit,
}

/// Port: a supermarket priced per standard unit.
#[async_trait]
pub trait StoreSource: Send + Sync {
    fn name(&self) -> &str;

    /// The product's price per standard unit. `Ok(None)` means the store does
    /// not sell it (or gives no comparable per-unit price); `Err` means the
    /// store could not be reached.
    async fn unit_price(&self, product: &str) -> anyhow::Result<Option<UnitPrice>>;
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

/// How one basket product compares across stores, priced per unit.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemComparison {
    pub name: String,
    pub quantity: u64,
    /// The unit the cheapest comparison is made in (the unit most stores
    /// report; ties go to the first store). `None` when no store priced it.
    pub unit: Option<Unit>,
    /// `(store name, per-unit price)` per store, in store order. `None` means
    /// the store gave no comparable price.
    pub per_store: Vec<(String, Option<UnitPrice>)>,
    /// The cheapest store among those priced in `unit`.
    pub cheapest: Option<String>,
}

/// The full result of comparing a basket across stores.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub items: Vec<ItemComparison>,
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

/// Extract a product size from a name, in canonical units (litres, kilograms,
/// pieces). Handles `1L`, `500ml`, `1kg`, `250g`, `Pk-12`. `None` when no
/// recognisable size is present.
pub fn parse_size(name: &str) -> Option<ItemSize> {
    let lower = name.to_lowercase().replace(',', ".");

    // Pack count: "pk-12" / "pack-12".
    if let Some(idx) = lower.find("pk-").or_else(|| lower.find("pack-")) {
        let rest: String = lower[idx..]
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(n) = rest.parse::<f64>() {
            if n > 0.0 {
                return Some(ItemSize { amount: n, unit: Unit::Each });
            }
        }
    }

    // A number directly followed by a unit suffix, scanning right to left so
    // the size token (usually last) wins over other digits in the name.
    let bytes: Vec<char> = lower.chars().collect();
    for (suffix, unit, divisor) in [
        ("ml", Unit::Litre, 1000.0),
        ("l", Unit::Litre, 1.0),
        ("kg", Unit::Kilogram, 1.0),
        ("g", Unit::Kilogram, 1000.0),
        ("ud", Unit::Each, 1.0),
    ] {
        if let Some(amount) = number_before(&bytes, suffix) {
            return Some(ItemSize { amount: amount / divisor, unit });
        }
    }
    None
}

/// Find a number immediately before `suffix` (optionally separated by a space).
fn number_before(chars: &[char], suffix: &str) -> Option<f64> {
    let s: String = chars.iter().collect();
    let suf: Vec<char> = suffix.chars().collect();
    for start in (0..chars.len()).rev() {
        if chars[start..].starts_with(&suf[..]) {
            // The char after the suffix must not be a letter (avoid "g" in "green").
            let after = chars.get(start + suf.len());
            if after.map(|c| c.is_alphabetic()).unwrap_or(false) {
                continue;
            }
            let mut i = start;
            let mut seen_digit = false;
            while i > 0 {
                let c = chars[i - 1];
                if c.is_ascii_digit() || c == '.' {
                    if c.is_ascii_digit() {
                        seen_digit = true;
                    }
                    i -= 1;
                } else if c == ' ' && i == start {
                    i -= 1; // allow one space between number and unit
                } else {
                    break;
                }
            }
            if seen_digit {
                let num: String = chars[i..start]
                    .iter()
                    .filter(|c| c.is_ascii_digit() || **c == '.')
                    .collect();
                if let Ok(n) = num.parse::<f64>() {
                    if n > 0.0 {
                        let _ = &s;
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

/// A per-unit price from a raw line price and the size parsed from a product
/// name. `None` when the name carries no recognisable size.
pub fn per_unit_from_name(price_cents: u64, name: &str) -> Option<UnitPrice> {
    parse_size(name).map(|s| UnitPrice {
        cents_per_unit: (price_cents as f64 / s.amount).round() as u64,
        unit: s.unit,
    })
}

/// Compare a typed basket string across stores, per unit.
pub async fn compare(
    stores: &[Box<dyn StoreSource>],
    basket: &str,
) -> Result<Comparison, CompareError> {
    let items = parse_basket(basket)?;
    Ok(compare_items(stores, &items).await)
}

/// Like [`compare`], for an already-parsed basket.
pub async fn compare_items(stores: &[Box<dyn StoreSource>], items: &[BasketItem]) -> Comparison {
    let mut item_comparisons = Vec::with_capacity(items.len());
    for item in items {
        let mut per_store = Vec::with_capacity(stores.len());
        for store in stores {
            let price = store.unit_price(&item.name).await.ok().flatten();
            per_store.push((store.name().to_string(), price));
        }
        let unit = comparison_unit(&per_store);
        let cheapest = cheapest_in_unit(&per_store, unit);
        item_comparisons.push(ItemComparison {
            name: item.name.clone(),
            quantity: item.quantity,
            unit,
            per_store,
            cheapest,
        });
    }
    Comparison { items: item_comparisons }
}

/// The unit to compare in: the one reported by the most stores, ties broken
/// by store order.
fn comparison_unit(per_store: &[(String, Option<UnitPrice>)]) -> Option<Unit> {
    let mut counts: Vec<(Unit, usize)> = Vec::new();
    for (_, price) in per_store {
        if let Some(p) = price {
            match counts.iter_mut().find(|(u, _)| *u == p.unit) {
                Some((_, n)) => *n += 1,
                None => counts.push((p.unit, 1)),
            }
        }
    }
    // Most-common unit; on a tie keep the first one (the earliest store).
    let mut best: Option<(Unit, usize)> = None;
    for (u, n) in counts {
        if best.map(|(_, bn)| n > bn).unwrap_or(true) {
            best = Some((u, n));
        }
    }
    best.map(|(u, _)| u)
}

/// The cheapest store among those priced in `unit`.
fn cheapest_in_unit(per_store: &[(String, Option<UnitPrice>)], unit: Option<Unit>) -> Option<String> {
    let unit = unit?;
    per_store
        .iter()
        .filter_map(|(name, price)| {
            price
                .filter(|p| p.unit == unit)
                .map(|p| (name, p.cents_per_unit))
        })
        .min_by_key(|(_, cents)| *cents)
        .map(|(name, _)| name.clone())
}
