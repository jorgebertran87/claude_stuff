use crate::basket::{BasketSource, FetchError};
use crate::comparer::{compare, compare_items, BasketItem, Comparison, StoreSource, StoreReport};

/// Build the bot's reply to a message.
///
/// A plain message is a typed basket: `milk x2, bread`, with an optional
/// `@ Store` suffix naming the store where it was bought. A `/command`
/// names a basket source instead: `/glovo` compares the latest order from
/// the source called "Glovo", `/glovo 1002` a specific one.
pub async fn reply_to(
    stores: &[Box<dyn StoreSource>],
    baskets: &[Box<dyn BasketSource>],
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
            Some(source) => order_reply(stores, source.as_ref(), argument).await,
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
    let (basket, bought_at) = match message.split_once('@') {
        Some((basket, store)) => (basket.trim(), Some(store.trim())),
        None => (message, None),
    };

    // Validate the bought store before spending time querying prices.
    if let Some(bought) = bought_at {
        if !stores.iter().any(|s| s.name().eq_ignore_ascii_case(bought)) {
            let known = stores.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ");
            return format!("{bought} is not a known store. Known stores: {known}.");
        }
    }

    let comparison = match compare(stores, basket).await {
        Ok(comparison) => comparison,
        Err(_) => return usage(),
    };

    let mut lines = vec![format!("🛒 {basket}"), String::new()];
    for (store, report) in &comparison.stores {
        lines.push(store_line(store, report, comparison.cheapest.as_deref()));
    }
    if let Some(bought) = bought_at {
        lines.push(String::new());
        lines.push(bought_lines(&comparison, bought));
    }
    lines.join("\n")
}

async fn order_reply(
    stores: &[Box<dyn StoreSource>],
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

    let comparison = compare_items(stores, &basket.items).await;
    let listing = basket.items.iter().map(item_label).collect::<Vec<_>>().join(", ");

    let mut lines = vec![format!("🛒 {} order: {listing}", source.name()), String::new()];
    for (store, report) in &comparison.stores {
        lines.push(store_line(store, report, comparison.cheapest.as_deref()));
    }
    if let Some(bought) = &basket.store {
        lines.push(String::new());
        // Unlike a typed basket, the order is already bought — an unknown
        // store is reported, not rejected.
        if comparison.stores.iter().any(|(store, _)| store.eq_ignore_ascii_case(bought)) {
            lines.push(bought_lines(&comparison, bought));
        } else {
            lines.push(format!("{bought} is not a compared store."));
        }
    }
    if let Some(paid) = basket.paid_cents {
        lines.push(format!("You paid {} on {}.", euros(paid), source.name()));
    }
    lines.join("\n")
}

fn item_label(item: &BasketItem) -> String {
    if item.quantity > 1 {
        format!("{} x{}", item.name, item.quantity)
    } else {
        item.name.clone()
    }
}

fn usage() -> String {
    "Send your basket as a comma-separated list, optionally with the store \
     where you bought it:\n  milk x2, bread @ Dia"
        .to_string()
}

fn euros(cents: u64) -> String {
    format!("{}.{:02} €", cents / 100, cents % 100)
}

fn store_line(store: &str, report: &StoreReport, cheapest: Option<&str>) -> String {
    match report {
        StoreReport::Complete { total_cents } => {
            let marker = if cheapest == Some(store) { "  ← cheapest" } else { "" };
            format!("{store}: {}{marker}", euros(*total_cents))
        }
        StoreReport::Incomplete { missing, .. } => {
            format!("{store}: incomplete (missing: {})", missing.join(", "))
        }
        StoreReport::Unavailable => format!("{store}: unavailable"),
    }
}

fn bought_lines(comparison: &Comparison, bought: &str) -> String {
    let report = comparison
        .stores
        .iter()
        .find(|(store, _)| store.eq_ignore_ascii_case(bought))
        .map(|(_, report)| report);

    let Some(StoreReport::Complete { total_cents: bought_total }) = report else {
        return format!(
            "Bought at {bought}: the total could not be compared \
             ({bought} did not price the full basket)."
        );
    };

    let cheapest_total = comparison.cheapest.as_deref().and_then(|cheapest| {
        comparison.stores.iter().find_map(|(store, report)| match report {
            StoreReport::Complete { total_cents } if store == cheapest => {
                Some((store.clone(), *total_cents))
            }
            _ => None,
        })
    });

    match cheapest_total {
        Some((cheapest, total)) if total < *bought_total => format!(
            "Bought at {bought}: {}\nYou could have saved {} buying at {cheapest}.",
            euros(*bought_total),
            euros(bought_total - total),
        ),
        _ => format!(
            "Bought at {bought}: {}\nYou bought at the cheapest store. 🎉",
            euros(*bought_total),
        ),
    }
}
