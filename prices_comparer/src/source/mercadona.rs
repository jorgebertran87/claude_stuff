use async_trait::async_trait;
use serde::Deserialize;

use crate::comparer::{
    brand_allows, choose_match, relevant, StoreMatch, StoreSource, Unit, UnitPrice,
};

use super::price::Price;

/// Algolia index backing tienda.mercadona.es product search.
/// The numeric part is the warehouse — prices vary slightly by region.
const SEARCH_INDEX: &str = "products_prod_4315_es";

/// Looks up prices on Mercadona's online shop, which serves product search
/// through Algolia. Among the hits it picks the cheapest priced in the wanted
/// measure, or the first when none is asked.
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
    #[serde(default)]
    display_name: String,
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

    async fn lookup(
        &self,
        product: &str,
        want: Option<Unit>,
    ) -> anyhow::Result<Option<StoreMatch>> {
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

        let mut candidates = Vec::with_capacity(search.hits.len());
        for hit in &search.hits {
            if !brand_allows(product, &hit.display_name) || !relevant(product, &hit.display_name) {
                continue;
            }
            let pi = &hit.price_instructions;
            if let (Some(price), Some(format)) = (&pi.reference_price, &pi.reference_format) {
                if let Some(unit) = unit_of(format) {
                    candidates.push(StoreMatch {
                        name: hit.display_name.clone(),
                        price: UnitPrice { cents_per_unit: price.to_cents()?, unit },
                    });
                }
            }
        }
        Ok(choose_match(candidates, want))
    }
}
