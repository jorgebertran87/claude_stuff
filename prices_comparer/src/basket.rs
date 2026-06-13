use async_trait::async_trait;

use crate::comparer::BasketItem;

/// One line of a purchased basket: what it was called and what it cost at the
/// source. The name may be a store-brand label that needs cleaning up before
/// it can be searched on the supermarkets (see [`OrderNormalizer`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchasedItem {
    pub name: String,
    pub quantity: u64,
    /// What this line cost at the source (e.g. the Glovo price), when known.
    pub price_cents: Option<u64>,
}

impl PurchasedItem {
    /// The comparison view of this line — just name and quantity.
    pub fn to_basket_item(&self) -> BasketItem {
        BasketItem { name: self.name.clone(), quantity: self.quantity }
    }
}

/// A basket that was actually bought somewhere, as recovered from an
/// external source (a Glovo order, a receipt, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchasedBasket {
    pub items: Vec<PurchasedItem>,
    /// The store it was bought at, when the source knows it.
    pub store: Option<String>,
    /// What was actually paid, when the source knows it.
    pub paid_cents: Option<u64>,
}

/// Port: turns a purchased basket's raw, store-branded line names into clean,
/// generic product names that search well on the supermarkets, keeping each
/// line's quantity and source price. Implemented over Claude in the
/// infrastructure layer; callers fall back to the raw items when it errors.
#[async_trait]
pub trait OrderNormalizer: Send + Sync {
    async fn normalize(&self, basket: &PurchasedBasket) -> anyhow::Result<Vec<PurchasedItem>>;
}

/// A normalizer that returns the basket's items unchanged. Used where no
/// cleanup is wanted (typed baskets) or as a stand-in in tests.
pub struct IdentityNormalizer;

#[async_trait]
impl OrderNormalizer for IdentityNormalizer {
    async fn normalize(&self, basket: &PurchasedBasket) -> anyhow::Result<Vec<PurchasedItem>> {
        Ok(basket.items.clone())
    }
}

/// Why a basket could not be fetched. Distinct variants so the bot can tell
/// the user exactly what to do — set a token, refresh it, or just retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// No credential is configured for the source yet.
    NotConfigured,
    /// The credential was rejected — typically an expired token.
    Unauthorized,
    /// The source could not be reached or returned unusable data.
    Unavailable,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::NotConfigured => write!(f, "not configured"),
            FetchError::Unauthorized => write!(f, "unauthorized"),
            FetchError::Unavailable => write!(f, "unavailable"),
        }
    }
}

impl std::error::Error for FetchError {}

/// Port: somewhere baskets can be read from instead of typed by hand.
/// Implementations live in the infrastructure layer (one adapter per source).
#[async_trait]
pub trait BasketSource: Send + Sync {
    fn name(&self) -> &str;

    /// Fetch a basket by order reference; `None` means the most recent one.
    /// `Ok(None)` when no matching order exists.
    async fn fetch_basket(
        &self,
        reference: Option<&str>,
    ) -> Result<Option<PurchasedBasket>, FetchError>;

    /// Update the source's credential at runtime. Sources without one
    /// reject this by default.
    async fn set_token(&self, _token: &str) -> anyhow::Result<()> {
        anyhow::bail!("{} has no token to configure", self.name())
    }
}
