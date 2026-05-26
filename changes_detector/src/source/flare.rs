use async_trait::async_trait;
use scraper::{Html, Selector};
use serde::Deserialize;

use super::Source;

/// Fetches a URL via FlareSolverr, which uses an undetected Chrome instance
/// to bypass Cloudflare JS challenges, then parses the returned HTML.
///
/// Requires a running FlareSolverr instance pointed to by `api_url`
/// (default: `http://flaresolverr:8191` in docker-compose).
pub struct FlareSolverSource {
    url: String,
    selector: Option<String>,
    mode: FetchMode,
    api_url: String,
    location: String,
    client: reqwest::Client,
}

/// What to return from the parsed HTML.
#[derive(Clone, Debug, PartialEq)]
pub enum FetchMode {
    /// Return the outer HTML of the first matching element (collapsed whitespace).
    Content,
    /// Return `"present"` or `"absent"` depending on whether the selector matches.
    Existence,
}

impl FlareSolverSource {
    pub fn new(
        url: String,
        selector: Option<String>,
        mode: FetchMode,
        api_url: String,
    ) -> Self {
        let location = match (&selector, &mode) {
            (Some(s), FetchMode::Existence) => format!("{url}  [{s}] (existence)"),
            (Some(s), _)                    => format!("{url}  [{s}]"),
            _                               => url.clone(),
        };
        Self { url, selector, mode, api_url, location, client: reqwest::Client::new() }
    }
}

// ---------------------------------------------------------------------------
// FlareSolverr API response DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct FlareSolverrResponse {
    solution: Solution,
}

#[derive(Deserialize)]
struct Solution {
    response: String,
}

// ---------------------------------------------------------------------------
// Source implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl Source for FlareSolverSource {
    fn location(&self) -> &str {
        &self.location
    }

    async fn fetch(&self) -> anyhow::Result<String> {
        let api_endpoint = format!("{}/v1", self.api_url);

        let resp = self.client
            .post(&api_endpoint)
            .json(&serde_json::json!({
                "cmd":        "request.get",
                "url":        self.url,
                "maxTimeout": 60000,
            }))
            .timeout(std::time::Duration::from_secs(90))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("FlareSolverr request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("FlareSolverr returned {status}: {body}");
        }

        let parsed: FlareSolverrResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse FlareSolverr response: {e}"))?;

        let html = parsed.solution.response;

        match &self.selector {
            // No selector — return the full rendered body text.
            None => {
                let doc = Html::parse_document(&html);
                let body_sel = Selector::parse("body").unwrap();
                let text = doc
                    .select(&body_sel)
                    .next()
                    .map(|el| el.text().collect::<Vec<_>>().join(" "))
                    .unwrap_or_default();
                Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
            }

            Some(selector_str) => {
                let sel = Selector::parse(selector_str)
                    .map_err(|e| anyhow::anyhow!("Invalid CSS selector '{selector_str}': {e:?}"))?;

                let doc = Html::parse_document(&html);
                let element = doc.select(&sel).next();

                match self.mode {
                    FetchMode::Content => {
                        let el = element.ok_or_else(|| {
                            anyhow::anyhow!(
                                "CSS selector '{selector_str}' matched no elements on {}",
                                self.url
                            )
                        })?;

                        // Reconstruct outer HTML with collapsed whitespace.
                        let tag   = el.value().name();
                        let attrs = el
                            .value()
                            .attrs()
                            .map(|(k, v)| format!("{k}=\"{v}\""))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let inner = el.inner_html();
                        let outer = if attrs.is_empty() {
                            format!("<{tag}>{inner}</{tag}>")
                        } else {
                            format!("<{tag} {attrs}>{inner}</{tag}>")
                        };

                        // Collapse whitespace for stable comparisons.
                        Ok(outer.split_whitespace().collect::<Vec<_>>().join(" "))
                    }

                    FetchMode::Existence => {
                        Ok(if element.is_some() {
                            "present".to_string()
                        } else {
                            "absent".to_string()
                        })
                    }
                }
            }
        }
    }
}
