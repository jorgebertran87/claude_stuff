use async_trait::async_trait;
use serde::Deserialize;

use crate::basket::{BasketSource, PurchasedBasket};
use crate::comparer::BasketItem;

use super::price::Price;

/// Reads baskets from the user's Glovo order history. Glovo has no public
/// API, so this talks to the endpoint the mobile app uses, authenticated
/// with the user's own bearer token.
pub struct GlovoSource {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl GlovoSource {
    /// `base_url` is `https://api.glovoapp.com` in production or a mock
    /// server in tests.
    pub fn new(base_url: String, token: String) -> Self {
        Self { client: reqwest::Client::new(), base_url, token }
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
    fn into_basket(self) -> anyhow::Result<PurchasedBasket> {
        let items = self
            .products
            .into_iter()
            .map(|p| BasketItem { name: p.name, quantity: p.quantity })
            .collect();
        let paid_cents = match &self.paid {
            Some(paid) => Some(paid.to_cents()?),
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
    ) -> anyhow::Result<Option<PurchasedBasket>> {
        let url = format!("{}/v3/customer/orders", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("authorization", &self.token)
            .send()
            .await?
            .error_for_status()?;
        let history: OrdersResponse = response.json().await?;

        // The history arrives newest first; no reference means the latest.
        let order = match reference {
            None => history.orders.into_iter().next(),
            Some(id) => history.orders.into_iter().find(|o| o.id.matches(id)),
        };
        order.map(Order::into_basket).transpose()
    }
}
