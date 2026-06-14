use async_trait::async_trait;
use serde::Deserialize;

use crate::comparer::{
    brand_allows, choose_unit_price, per_unit_from_name, StoreSource, Unit, UnitPrice,
};

use super::price::Price;

/// Dia's product search endpoint. Cloudflare-protected, so it is fetched
/// through FlareSolverr rather than directly.
const SEARCH_URL: &str = "https://www.dia.es/api/v1/search-back/search/reduced";

/// Looks up prices on Dia's online shop via FlareSolverr, which drives an
/// undetected Chrome to get past Cloudflare. Among the search results it picks
/// the cheapest priced in the wanted measure, or the first when none is asked.
pub struct DiaSource {
    client: reqwest::Client,
    flare_url: String,
}

impl DiaSource {
    /// `flare_url` is the FlareSolverr instance, e.g. `http://flaresolverr:8191`
    /// in docker-compose or a mock server in tests.
    pub fn new(flare_url: String) -> Self {
        Self { client: reqwest::Client::new(), flare_url }
    }
}

#[derive(Deserialize)]
struct FlareSolverrResponse {
    solution: Solution,
}

#[derive(Deserialize)]
struct Solution {
    response: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    search_items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default, alias = "display_name")]
    name: String,
    prices: Prices,
}

#[derive(Deserialize)]
struct Prices {
    price: Price,
}

/// A browser renders a JSON URL wrapped in `<pre>` HTML; FlareSolverr hands
/// back that rendering. Unwrap it, or pass bare JSON through untouched.
fn embedded_json(response: &str) -> &str {
    let Some(start) = response.find("<pre") else {
        return response.trim();
    };
    let Some(open_end) = response[start..].find('>') else {
        return response.trim();
    };
    let after = &response[start + open_end + 1..];
    match after.find("</pre>") {
        Some(end) => after[..end].trim(),
        None => after.trim(),
    }
}

#[async_trait]
impl StoreSource for DiaSource {
    fn name(&self) -> &str {
        "Dia"
    }

    async fn unit_price(
        &self,
        product: &str,
        want: Option<Unit>,
    ) -> anyhow::Result<Option<UnitPrice>> {
        let mut search_url = reqwest::Url::parse(SEARCH_URL)?;
        search_url.query_pairs_mut().append_pair("q", product);

        let response = self
            .client
            .post(format!("{}/v1", self.flare_url))
            .json(&serde_json::json!({
                "cmd":        "request.get",
                "url":        search_url.as_str(),
                "maxTimeout": 60000,
            }))
            .timeout(std::time::Duration::from_secs(90))
            .send()
            .await?
            .error_for_status()?;

        let parsed: FlareSolverrResponse = response.json().await?;
        let search: SearchResponse = serde_json::from_str(embedded_json(&parsed.solution.response))
            .map_err(|e| anyhow::anyhow!("Dia search returned no product data: {e}"))?;

        let mut candidates = Vec::with_capacity(search.search_items.len());
        for item in &search.search_items {
            if !brand_allows(product, &item.name) {
                continue;
            }
            if let Some(price) = per_unit_from_name(item.prices.price.to_cents()?, &item.name) {
                candidates.push(price);
            }
        }
        Ok(choose_unit_price(candidates, want))
    }
}
