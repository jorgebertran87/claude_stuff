use async_trait::async_trait;
use serde::Deserialize;

use crate::comparer::StoreSource;

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
    unit_price: UnitPrice,
}

/// Mercadona serves the unit price as a decimal string in some endpoints and
/// as a JSON number in others; accept both.
#[derive(Deserialize)]
#[serde(untagged)]
enum UnitPrice {
    Text(String),
    Number(f64),
}

impl UnitPrice {
    fn to_cents(&self) -> anyhow::Result<u64> {
        match self {
            UnitPrice::Number(euros) => Ok((euros * 100.0).round() as u64),
            UnitPrice::Text(text) => {
                let (euros, cents) = text.split_once('.').unwrap_or((text, "0"));
                let euros: u64 = euros.parse()?;
                let cents: u64 = format!("{cents:0<2}")[..2].parse()?;
                Ok(euros * 100 + cents)
            }
        }
    }
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
