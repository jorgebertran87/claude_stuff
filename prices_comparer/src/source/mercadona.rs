use async_trait::async_trait;
use serde::Deserialize;

use crate::comparer::StoreSource;

use super::price::Price;

/// Algolia index backing tienda.mercadona.es product search.
/// The numeric part is the warehouse — prices vary slightly by region.
const SEARCH_INDEX: &str = "products_prod_4315_es";

/// Looks up prices on Mercadona's online shop, which serves product search
/// through Algolia. The first hit for a query is taken as "the product".
pub struct MercadonaSource {
    client: reqwest::Client,
    base_url: String,
    app_id: String,
    api_key: String,
}

impl MercadonaSource {
    /// `base_url` is the Algolia host, e.g. `https://7uzjkl1dj0-dsn.algolia.net`
    /// in production or a mock server in tests.
    pub fn new(base_url: String, app_id: String, api_key: String) -> Self {
        Self { client: reqwest::Client::new(), base_url, app_id, api_key }
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
    price_instructions: PriceInstructions,
}

#[derive(Deserialize)]
struct PriceInstructions {
    unit_price: Price,
}

#[async_trait]
impl StoreSource for MercadonaSource {
    fn name(&self) -> &str {
        "Mercadona"
    }

    async fn price_cents(&self, product: &str) -> anyhow::Result<Option<u64>> {
        let url = format!("{}/1/indexes/{}/query", self.base_url, SEARCH_INDEX);
        let response = self
            .client
            .post(&url)
            .header("x-algolia-application-id", &self.app_id)
            .header("x-algolia-api-key", &self.api_key)
            .json(&serde_json::json!({ "query": product }))
            .send()
            .await?
            .error_for_status()?;
        let search: SearchResponse = response.json().await?;
        match search.hits.first() {
            None => Ok(None),
            Some(hit) => Ok(Some(hit.price_instructions.unit_price.to_cents()?)),
        }
    }
}
