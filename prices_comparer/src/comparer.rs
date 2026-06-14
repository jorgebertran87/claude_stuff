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

    /// Long-form measure name: "litre", "kilo", "each".
    pub fn measure(&self) -> &'static str {
        match self {
            Unit::Litre => "litre",
            Unit::Kilogram => "kilo",
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

/// A store's matched product: the name it returned for a query and its
/// per-unit price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreMatch {
    pub name: String,
    pub price: UnitPrice,
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

    /// The matched product and its price per standard unit. `want` is the
    /// measure to compare in (from the order being priced): when set, the store
    /// picks the cheapest matching option among its search results; when `None`,
    /// it takes the first result. `Ok(None)` means the store does not sell it
    /// (or gives no comparable per-unit price); `Err` means the store could not
    /// be reached.
    async fn lookup(
        &self,
        product: &str,
        want: Option<Unit>,
    ) -> anyhow::Result<Option<StoreMatch>>;
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
    /// `(store name, matched product)` per store, in store order. `None` means
    /// the store gave no comparable price.
    pub per_store: Vec<(String, Option<StoreMatch>)>,
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

    // Pack count: "pk-12" / "pack-12". A pack of a sized product
    // ("20ML, Pk-12") holds pack × size, so we fold it into the measured size
    // below; on its own a pack count is just a number of pieces.
    let pack = parse_pack_count(&lower);

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
            let amount = amount / divisor;
            // A measured size in a pack is held once per piece, so the pack
            // holds pack × size in total.
            let amount = if unit == Unit::Each { amount } else { amount * pack.unwrap_or(1.0) };
            return Some(ItemSize { amount, unit });
        }
    }

    // No measured size: a bare pack count is a plain piece count.
    pack.map(|amount| ItemSize { amount, unit: Unit::Each })
}

/// The positive pack count in a "pk-12" / "pack-12" token, if present.
fn parse_pack_count(lower: &str) -> Option<f64> {
    let idx = lower.find("pk-").or_else(|| lower.find("pack-"))?;
    let digits: String = lower[idx..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<f64>().ok().filter(|n| *n > 0.0)
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

/// Some generic queries must always resolve to a specific brand. The brand
/// keyword a query's results must contain, if any.
fn required_brand(query: &str) -> Option<&'static str> {
    match query.trim().to_lowercase().as_str() {
        "cola" | "cola zero" => Some("coca"),
        _ => None,
    }
}

/// Whether a product `name` is eligible for `query`. A generic cola query only
/// allows the Coca-Cola brand; any other query allows every result.
pub fn brand_allows(query: &str, name: &str) -> bool {
    match required_brand(query) {
        Some(brand) => name.to_lowercase().contains(brand),
        None => true,
    }
}

/// Pick the product a store should report from its search results
/// (`candidates`, in result order). With a wanted measure, the cheapest match
/// in that unit wins; without one, the first result wins.
pub fn choose_match(
    candidates: impl IntoIterator<Item = StoreMatch>,
    want: Option<Unit>,
) -> Option<StoreMatch> {
    match want {
        Some(unit) => candidates
            .into_iter()
            .filter(|m| m.price.unit == unit)
            .min_by_key(|m| m.price.cents_per_unit),
        None => candidates.into_iter().next(),
    }
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
    compare_with_anchors(stores, items, "", &[]).await
}

/// Like [`compare_items`], but each line may carry an `anchor` price from the
/// order source itself (labelled `anchor_label`, e.g. "Glovo"), aligned with
/// `items` by index. The anchor joins the comparison like a store — it can be
/// the cheapest — and, when present, its unit drives the comparison so every
/// store is judged in the same terms the item was bought in.
pub async fn compare_with_anchors(
    stores: &[Box<dyn StoreSource>],
    items: &[BasketItem],
    anchor_label: &str,
    anchors: &[Option<StoreMatch>],
) -> Comparison {
    let mut item_comparisons = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let anchor = anchors.get(i).cloned().flatten();
        // Match each store's option to the way the item was bought, so the
        // cheapest comparable size is chosen rather than the first search hit.
        let want = anchor.as_ref().map(|a| a.price.unit);
        let mut per_store = Vec::with_capacity(stores.len() + 1);
        if let Some(matched) = &anchor {
            per_store.push((anchor_label.to_string(), Some(matched.clone())));
        }
        for store in stores {
            let matched = store.lookup(&item.name, want).await.ok().flatten();
            per_store.push((store.name().to_string(), matched));
        }
        // The order's own measurement wins when it has one, so the stores are
        // compared the way the item was actually bought; otherwise fall back to
        // the unit most stores report.
        let unit = want.or_else(|| comparison_unit(&per_store));
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

/// The unit to compare in. Volume and weight always win over per-piece: a
/// product is compared by litre or kilo whenever any store prices it that way,
/// and falls back to `Each` only when no store gives a volume/weight price.
/// Within the chosen tier the unit reported by the most stores wins, ties
/// broken by store order.
fn comparison_unit(per_store: &[(String, Option<StoreMatch>)]) -> Option<Unit> {
    let is_measured = |u: Unit| matches!(u, Unit::Litre | Unit::Kilogram);
    let has_measured = per_store
        .iter()
        .any(|(_, matched)| matched.as_ref().is_some_and(|m| is_measured(m.price.unit)));

    let mut counts: Vec<(Unit, usize)> = Vec::new();
    for (_, matched) in per_store {
        if let Some(m) = matched {
            let unit = m.price.unit;
            // Once any store prices by volume/weight, per-piece prices no
            // longer get a say in which unit we compare in.
            if has_measured && !is_measured(unit) {
                continue;
            }
            match counts.iter_mut().find(|(u, _)| *u == unit) {
                Some((_, n)) => *n += 1,
                None => counts.push((unit, 1)),
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
fn cheapest_in_unit(per_store: &[(String, Option<StoreMatch>)], unit: Option<Unit>) -> Option<String> {
    let unit = unit?;
    per_store
        .iter()
        .filter_map(|(name, matched)| {
            matched
                .as_ref()
                .filter(|m| m.price.unit == unit)
                .map(|m| (name, m.price.cents_per_unit))
        })
        .min_by_key(|(_, cents)| *cents)
        .map(|(name, _)| name.clone())
}
