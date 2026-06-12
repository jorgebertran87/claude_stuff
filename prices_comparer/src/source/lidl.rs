use async_trait::async_trait;
use serde::Deserialize;

use crate::comparer::StoreSource;

use super::price::Price;

/// Looks up prices on Lidl's online shop search API. The first item for a
/// query is taken as "the product".
pub struct LidlSource {
    client: reqwest::Client,
    base_url: String,
}

impl LidlSource {
    /// `base_url` is the shop host, e.g. `https://www.lidl.es` in production
    /// or a mock server in tests.
    pub fn new(base_url: String) -> Self {
        Self { client: reqwest::Client::new(), base_url }
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    gridbox: Gridbox,
}

#[derive(Deserialize)]
struct Gridbox {
    data: GridboxData,
}

#[derive(Deserialize)]
struct GridboxData {
    price: ItemPrice,
}

#[derive(Deserialize)]
struct ItemPrice {
    price: Price,
}

#[async_trait]
impl StoreSource for LidlSource {
    fn name(&self) -> &str {
        "Lidl"
    }

    async fn price_cents(&self, product: &str) -> anyhow::Result<Option<u64>> {
        let mut url = reqwest::Url::parse(&format!("{}/q/api/search", self.base_url))?;
        url.query_pairs_mut()
            .append_pair("q", product)
            .append_pair("variant", "default");

        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?;
        let search: SearchResponse = response.json().await?;
        match search.items.first() {
            None => Ok(None),
            Some(item) => Ok(Some(item.gridbox.data.price.price.to_cents()?)),
        }
    }
}
