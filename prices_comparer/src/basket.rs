use async_trait::async_trait;

use crate::comparer::BasketItem;

/// A basket that was actually bought somewhere, as recovered from an
/// external source (a Glovo order, a receipt, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchasedBasket {
    pub items: Vec<BasketItem>,
    /// The store it was bought at, when the source knows it.
    pub store: Option<String>,
    /// What was actually paid, when the source knows it.
    pub paid_cents: Option<u64>,
}

/// Port: somewhere baskets can be read from instead of typed by hand.
/// Implementations live in the infrastructure layer (one adapter per source).
#[async_trait]
pub trait BasketSource: Send + Sync {
    fn name(&self) -> &str;

    /// Fetch a basket by order reference; `None` means the most recent one.
    /// `Ok(None)` when no matching order exists.
    async fn fetch_basket(&self, reference: Option<&str>)
        -> anyhow::Result<Option<PurchasedBasket>>;
}
