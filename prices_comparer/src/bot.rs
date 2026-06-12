use crate::comparer::{compare, Comparison, StoreSource, StoreReport};

/// Build the bot's reply to a basket message.
///
/// Message format: `milk x2, bread` with an optional `@ Store` suffix naming
/// the store where the basket was bought, e.g. `milk x2, bread @ Dia`.
pub async fn reply_to(stores: &[Box<dyn StoreSource>], message: &str) -> String {
    let (basket, bought_at) = match message.split_once('@') {
        Some((basket, store)) => (basket.trim(), Some(store.trim())),
        None => (message.trim(), None),
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
