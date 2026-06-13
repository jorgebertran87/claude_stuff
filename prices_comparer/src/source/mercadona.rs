use async_trait::async_trait;
use serde::Deserialize;

use crate::comparer::{StoreSource, Unit, UnitPrice};

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
    /// Price per reference unit, e.g. "0.960".
    reference_price: Option<Price>,
    /// The reference unit, e.g. "L", "kg", "ud".
    reference_format: Option<String>,
}

/// Map Mercadona's reference_format to a comparison unit.
fn unit_of(format: &str) -> Option<Unit> {
    match format.trim().to_lowercase().as_str() {
        "l" => Some(Unit::Litre),
        "kg" => Some(Unit::Kilogram),
        "ud" | "u" | "unidad" => Some(Unit::Each),
        _ => None,
    }
}

#[async_trait]
impl StoreSource for MercadonaSource {
    fn name(&self) -> &str {
        "Mercadona"
    }

    async fn unit_price(&self, product: &str) -> anyhow::Result<Option<UnitPrice>> {
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
        let Some(hit) = search.hits.first() else {
            return Ok(None);
        };
        let pi = &hit.price_instructions;
        match (&pi.reference_price, &pi.reference_format) {
            (Some(price), Some(format)) => match unit_of(format) {
                Some(unit) => Ok(Some(UnitPrice { cents_per_unit: price.to_cents()?, unit })),
                None => Ok(None),
            },
            _ => Ok(None),
        }
    }
}
