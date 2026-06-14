use crate::basket::{BasketSource, FetchError, OrderNormalizer, PurchasedItem};
use crate::comparer::{
    compare, compare_with_anchors, BasketItem, ItemComparison, ItemSize, StoreSource, UnitPrice,
};

/// Build the bot's reply to a message.
///
/// A plain message is a typed basket (`milk, bread`). A `/command` names a
/// basket source instead: `/glovo` compares the latest order from the source
/// called "Glovo", `/glovo jamon` the latest from a matching store.
///
/// Orders from a source are normalized (store-brand names cleaned up) before
/// the comparison; typed baskets are compared as written. Every product is
/// priced per unit (€/L, €/kg, €/each) and the cheapest is marked. For an
/// order, the price paid on the source competes as its own column, and its
/// measurement decides the unit the stores are compared in.
pub async fn reply_to(
    stores: &[Box<dyn StoreSource>],
    baskets: &[Box<dyn BasketSource>],
    normalizer: &dyn OrderNormalizer,
    message: &str,
) -> String {
    let trimmed = message.trim();
    if let Some(command) = trimmed.strip_prefix('/') {
        let (word, argument) = match command.split_once(' ') {
            Some((word, argument)) => (word.trim(), Some(argument.trim())),
            None => (command.trim(), None),
        };
        // "/<source>_token <value>" sets that source's credential.
        if let Some(name) = word.strip_suffix("_token") {
            return match find_source(baskets, name) {
                Some(source) => set_token_reply(source.as_ref(), argument).await,
                None => usage(),
            };
        }
        // "/<source> [reference]" compares an order from that source.
        return match find_source(baskets, word) {
            Some(source) => order_reply(stores, normalizer, source.as_ref(), argument).await,
            None => usage(),
        };
    }
    typed_reply(stores, trimmed).await
}

fn find_source<'a>(
    baskets: &'a [Box<dyn BasketSource>],
    name: &str,
) -> Option<&'a Box<dyn BasketSource>> {
    baskets.iter().find(|b| b.name().eq_ignore_ascii_case(name))
}

async fn set_token_reply(source: &dyn BasketSource, token: Option<&str>) -> String {
    let Some(token) = token.filter(|t| !t.is_empty()) else {
        return format!("Send the token after the command:\n  /{}_token <token>", source.name().to_lowercase());
    };
    match source.set_token(token).await {
        Ok(()) => format!("{} token saved. ✅", source.name()),
        Err(e) => format!("Could not save the {} token: {e}", source.name()),
    }
}

async fn typed_reply(stores: &[Box<dyn StoreSource>], message: &str) -> String {
    let basket = message.split('@').next().unwrap_or(message).trim();
    let comparison = match compare(stores, basket).await {
        Ok(comparison) => comparison,
        Err(_) => return usage(),
    };

    let mut lines = vec![format!("🛒 {basket}"), String::new()];
    for item in &comparison.items {
        lines.push(typed_item_line(item));
    }
    lines.join("\n")
}

async fn order_reply(
    stores: &[Box<dyn StoreSource>],
    normalizer: &dyn OrderNormalizer,
    source: &dyn BasketSource,
    reference: Option<&str>,
) -> String {
    let name = source.name();
    let basket = match source.fetch_basket(reference).await {
        Err(FetchError::NotConfigured) => return format!(
            "{name} is not configured. Send the bearer token with:\n  /{}_token <token>\n\
             or set up automatic capture so it stays fresh.",
            name.to_lowercase(),
        ),
        Err(FetchError::Unauthorized) => return format!(
            "The {name} token has expired. Open the {name} app to refresh it \
             (automatic capture will pick it up), or send a new one with /{}_token <token>.",
            name.to_lowercase(),
        ),
        Err(FetchError::Unavailable) => {
            return format!("{name} could not be reached. Try again later.")
        }
        Ok(None) => return format!("No {name} order was found."),
        Ok(Some(basket)) => basket,
    };
    if basket.items.is_empty() {
        return format!("The {} order has no products.", source.name());
    }

    // Clean the store-brand names so they match supermarket search; fall back
    // to the raw items if normalization fails so the order still gets compared.
    let items = match normalizer.normalize(&basket).await {
        Ok(clean) if !clean.is_empty() => clean,
        _ => basket.items.clone(),
    };

    let comparison_items: Vec<BasketItem> = items.iter().map(|i| i.to_basket_item()).collect();
    // The price paid on the source competes in the comparison as its own column.
    let anchors: Vec<Option<UnitPrice>> = items.iter().map(glovo_unit_price).collect();
    let comparison = compare_with_anchors(stores, &comparison_items, source.name(), &anchors).await;

    let mut lines = vec![format!("🛒 {} order:", source.name()), String::new()];
    for (i, item) in comparison.items.iter().enumerate() {
        lines.push(order_item_line(item, &items[i]));
    }
    if let Some(paid) = basket.paid_cents {
        lines.push(String::new());
        lines.push(format!("You paid {} on {}.", euros(paid), source.name()));
    }
    lines.join("\n")
}

// ── Rendering ──────────────────────────────────────────────────────────────

fn typed_item_line(item: &ItemComparison) -> String {
    let qty = if item.quantity > 1 { format!(" x{}", item.quantity) } else { String::new() };
    format!("{}{qty} — {}", item.name, store_cells(item))
}

fn order_item_line(item: &ItemComparison, purchased: &PurchasedItem) -> String {
    let size = purchased
        .size
        .map(|s| format!(" ({})", size_label(s)))
        .unwrap_or_default();
    format!("{}{size} — {}", item.name, store_cells(item))
}

/// Each store's per-unit price, with the cheapest marked and a dash for
/// stores that gave no comparable price.
fn store_cells(item: &ItemComparison) -> String {
    item.per_store
        .iter()
        .map(|(store, price)| match price {
            Some(p) => {
                let mark = if item.cheapest.as_deref() == Some(store.as_str()) {
                    " ← cheapest"
                } else {
                    ""
                };
                format!("{store} {}{mark}", unit_price_str(p))
            }
            None => format!("{store} —"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The source price per unit (line price / size), when both are known.
fn glovo_unit_price(p: &PurchasedItem) -> Option<UnitPrice> {
    match (p.price_cents, p.size) {
        (Some(cents), Some(size)) if size.amount > 0.0 => Some(UnitPrice {
            cents_per_unit: (cents as f64 / size.amount).round() as u64,
            unit: size.unit,
        }),
        _ => None,
    }
}

fn unit_price_str(p: &UnitPrice) -> String {
    format!("{}/{}", euros(p.cents_per_unit), p.unit.label())
}

fn size_label(s: ItemSize) -> String {
    let amount = if s.amount.fract() == 0.0 {
        format!("{}", s.amount as u64)
    } else {
        format!("{}", s.amount)
    };
    format!("{amount} {}", s.unit.label())
}

fn usage() -> String {
    "Send your basket as a comma-separated list:\n  milk, bread, eggs".to_string()
}

fn euros(cents: u64) -> String {
    format!("{}.{:02} €", cents / 100, cents % 100)
}
