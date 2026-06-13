use async_trait::async_trait;
use serde::Deserialize;

use crate::basket::{BasketSource, FetchError, PurchasedBasket, PurchasedItem};
use crate::token_store::TokenStore;

/// Glovo API version the web client sends; required alongside the other
/// `glovo-app-*` context headers or the API answers 404.
const APP_VERSION: &str = "v1.2329.0";

/// Reads baskets from the user's Glovo order history. Glovo has no public
/// API, so this talks to the endpoints its web client uses, authenticated
/// with the user's own bearer token (sent raw, without a `Bearer` prefix).
///
/// A basket needs two calls: the orders list resolves an order id (the most
/// recent, or the most recent whose store name matches a search word), and
/// the order detail gives the structured line items, store and total.
///
/// The token is read from a shared [`TokenStore`] on every request, so a
/// token captured from live traffic (or set with `/glovo_token`) takes
/// effect without a restart.
pub struct GlovoSource {
    client: reqwest::Client,
    base_url: String,
    tokens: TokenStore,
}

impl GlovoSource {
    /// `base_url` is `https://api.glovoapp.com` in production or a mock
    /// server in tests.
    pub fn new(base_url: String, tokens: TokenStore) -> Self {
        Self { client: reqwest::Client::new(), base_url, tokens }
    }

    /// GET with the headers the Glovo web client sends. `None` status 401/403
    /// maps to `Unauthorized`; transport errors map to `Unavailable`.
    async fn get(&self, url: &str, token: &str) -> Result<reqwest::Response, FetchError> {
        let response = self
            .client
            .get(url)
            .header("authorization", token)
            .header("accept", "application/json, text/plain, */*")
            .header("glovo-api-version", "14")
            .header("glovo-app-context", "web")
            .header("glovo-app-platform", "web")
            .header("glovo-app-type", "customer")
            .header("glovo-app-version", APP_VERSION)
            .send()
            .await
            .map_err(|_| FetchError::Unavailable)?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(FetchError::Unauthorized);
        }
        Ok(response)
    }
}

// ── API response DTOs ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct OrdersList {
    orders: Vec<OrderSummary>,
}

#[derive(Deserialize)]
struct OrderSummary {
    #[serde(rename = "orderId")]
    order_id: i64,
    content: Option<OrderContent>,
}

#[derive(Deserialize)]
struct OrderContent {
    title: Option<String>,
}

impl OrderSummary {
    /// The store name as shown on the order card.
    fn store_title(&self) -> &str {
        self.content.as_ref().and_then(|c| c.title.as_deref()).unwrap_or("")
    }
}

#[derive(Deserialize)]
struct OrderDetail {
    #[serde(rename = "storeName")]
    store_name: Option<String>,
    #[serde(rename = "boughtProducts")]
    bought_products: Vec<BoughtProduct>,
    #[serde(rename = "pricingBreakdown")]
    pricing_breakdown: PricingBreakdown,
}

#[derive(Deserialize)]
struct BoughtProduct {
    name: String,
    /// A display string like `"1x"`.
    quantity: String,
    /// A display amount like `"1,95 €"`, when present.
    #[serde(default)]
    price: Option<String>,
}

#[derive(Deserialize)]
struct PricingBreakdown {
    lines: Vec<PriceLine>,
}

#[derive(Deserialize)]
struct PriceLine {
    #[serde(rename = "type")]
    line_type: String,
    /// A display amount like `"34,07 €"` (or `"No cost"` for free lines).
    amount: String,
}

impl OrderDetail {
    fn into_basket(self) -> PurchasedBasket {
        let items = self
            .bought_products
            .into_iter()
            .map(|p| PurchasedItem {
                name: p.name,
                quantity: parse_quantity(&p.quantity),
                price_cents: p.price.as_deref().and_then(parse_euros),
            })
            .collect();
        let paid_cents = self
            .pricing_breakdown
            .lines
            .iter()
            .find(|l| l.line_type == "TOTAL")
            .and_then(|l| parse_euros(&l.amount));
        PurchasedBasket { items, store: self.store_name, paid_cents }
    }
}

/// Lowercase and strip common Spanish accents, for accent-insensitive
/// store-name matching ("jamon" matches "Jamón").
fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            'ç' | 'Ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// Leading integer of a `"1x"`-style quantity; at least 1.
fn parse_quantity(raw: &str) -> u64 {
    raw.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(1)
        .max(1)
}

/// Parse a Spanish-format euro amount like `"34,07 €"` or `"1.234,56 €"` into
/// cents. When a comma is present it is the decimal separator and `.` is the
/// thousands separator; otherwise the string is treated as a plain number.
/// Returns `None` for non-numeric amounts (e.g. `"No cost"`).
fn parse_euros(raw: &str) -> Option<u64> {
    let kept: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .collect();
    let normalized = if kept.contains(',') {
        kept.replace('.', "").replace(',', ".")
    } else {
        kept
    };
    if normalized.is_empty() {
        return None;
    }
    let value: f64 = normalized.parse().ok()?;
    Some((value * 100.0).round() as u64)
}

// ── BasketSource ────────────────────────────────────────────────────────────

#[async_trait]
impl BasketSource for GlovoSource {
    fn name(&self) -> &str {
        "Glovo"
    }

    async fn fetch_basket(
        &self,
        reference: Option<&str>,
    ) -> Result<Option<PurchasedBasket>, FetchError> {
        let token = self.tokens.current().ok_or(FetchError::NotConfigured)?;

        // Fetch recent orders and pick one: the latest, or — when a word is
        // given — the latest whose store name matches it (accent-insensitive).
        let url = format!("{}/v3/customer/orders-list?offset=0&limit=20", self.base_url);
        let response = self
            .get(&url, &token)
            .await?
            .error_for_status()
            .map_err(|_| FetchError::Unavailable)?;
        let list: OrdersList = response.json().await.map_err(|_| FetchError::Unavailable)?;

        let order = match reference {
            None => list.orders.into_iter().next(),
            Some(word) => {
                let needle = fold(word);
                list.orders.into_iter().find(|o| fold(o.store_title()).contains(&needle))
            }
        };
        let order_id = match order {
            Some(o) => o.order_id,
            None => return Ok(None),
        };

        // Fetch the structured detail for that order.
        let url = format!("{}/v3/customer/orders/{order_id}", self.base_url);
        let response = self.get(&url, &token).await?;
        // An unknown order id is "no order found", not a transport failure.
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        let response = response.error_for_status().map_err(|_| FetchError::Unavailable)?;
        let detail: OrderDetail = response.json().await.map_err(|_| FetchError::Unavailable)?;
        Ok(Some(detail.into_basket()))
    }

    async fn set_token(&self, token: &str) -> anyhow::Result<()> {
        self.tokens.set(token)
    }
}
