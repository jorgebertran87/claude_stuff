use async_trait::async_trait;
use scraper::{Html, Selector};

use super::Source;

/// Fetches an HTTP/HTTPS URL and returns either the full response body or,
/// when a CSS selector is supplied, the concatenated text content of every
/// matching element.
pub struct HttpSource {
    url: String,
    selector: Option<Selector>,
    /// Pre-built display string used in Telegram notifications.
    location: String,
}

impl HttpSource {
    /// `selector_str` is an optional CSS selector, e.g. `"a[id=\"237\"]"`.
    /// Returns an error if the selector string is present but invalid.
    pub fn new(url: String, selector_str: Option<&str>) -> anyhow::Result<Self> {
        let (selector, location) = match selector_str {
            None => (None, url.clone()),
            Some(s) => {
                let parsed = Selector::parse(s)
                    .map_err(|e| anyhow::anyhow!("Invalid CSS selector '{s}': {e:?}"))?;
                let location = format!("{url}  [{s}]");
                (Some(parsed), location)
            }
        };

        Ok(Self { url, selector, location })
    }
}

#[async_trait]
impl Source for HttpSource {
    fn location(&self) -> &str {
        &self.location
    }

    async fn fetch(&self) -> anyhow::Result<String> {
        let body = reqwest::get(&self.url)
            .await
            .map_err(|e| anyhow::anyhow!("GET {} failed: {e}", self.url))?
            .error_for_status()
            .map_err(|e| anyhow::anyhow!("GET {} returned error status: {e}", self.url))?
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body from {}: {e}", self.url))?;

        match &self.selector {
            // No selector — monitor the whole response body.
            None => Ok(body),

            // Selector present — extract text of every matching element.
            Some(selector) => {
                let document = Html::parse_document(&body);

                let text: Vec<String> = document
                    .select(selector)
                    .map(|el| {
                        // Collect all descendant text nodes, collapse whitespace.
                        el.text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .filter(|s| !s.is_empty())
                    .collect();

                if text.is_empty() {
                    anyhow::bail!(
                        "CSS selector matched no elements on {}",
                        self.url
                    );
                }

                Ok(text.join("\n"))
            }
        }
    }
}
