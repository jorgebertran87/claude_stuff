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
