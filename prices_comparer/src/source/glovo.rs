use async_trait::async_trait;
use serde::Deserialize;

use crate::basket::{BasketSource, FetchError, PurchasedBasket};
use crate::comparer::BasketItem;
use crate::token_store::TokenStore;

use super::price::Price;

/// Reads baskets from the user's Glovo order history. Glovo has no public
/// API, so this talks to the endpoint the mobile app uses, authenticated
/// with the user's own bearer token.
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
}

#[derive(Deserialize)]
struct OrdersResponse {
    orders: Vec<Order>,
}

#[derive(Deserialize)]
struct Order {
    id: OrderId,
    #[serde(alias = "partner_name", alias = "partnerName")]
    store_name: Option<String>,
    #[serde(alias = "total")]
    paid: Option<Price>,
    products: Vec<Product>,
}

#[derive(Deserialize)]
struct Product {
    name: String,
    #[serde(default = "one")]
    quantity: u64,
}

fn one() -> u64 {
    1
}

/// Order ids arrive as numbers or strings depending on the endpoint.
#[derive(Deserialize)]
#[serde(untagged)]
enum OrderId {
    Number(i64),
    Text(String),
}

impl OrderId {
    fn matches(&self, reference: &str) -> bool {
        match self {
            OrderId::Number(n) => n.to_string() == reference,
            OrderId::Text(s) => s == reference,
        }
    }
}

impl Order {
    fn into_basket(self) -> Result<PurchasedBasket, FetchError> {
        let items = self
            .products
            .into_iter()
            .map(|p| BasketItem { name: p.name, quantity: p.quantity })
            .collect();
        let paid_cents = match &self.paid {
            Some(paid) => Some(paid.to_cents().map_err(|_| FetchError::Unavailable)?),
            None => None,
        };
        Ok(PurchasedBasket { items, store: self.store_name, paid_cents })
    }
}

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

        let url = format!("{}/v3/customer/orders", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("authorization", token)
            .send()
            .await
            .map_err(|_| FetchError::Unavailable)?;

        // A rejected token is the common case worth its own message.
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(FetchError::Unauthorized);
        }
        let response = response.error_for_status().map_err(|_| FetchError::Unavailable)?;
        let history: OrdersResponse =
            response.json().await.map_err(|_| FetchError::Unavailable)?;

        // The history arrives newest first; no reference means the latest.
        let order = match reference {
            None => history.orders.into_iter().next(),
            Some(id) => history.orders.into_iter().find(|o| o.id.matches(id)),
        };
        order.map(Order::into_basket).transpose()
    }

    async fn set_token(&self, token: &str) -> anyhow::Result<()> {
        self.tokens.set(token)
    }
}
