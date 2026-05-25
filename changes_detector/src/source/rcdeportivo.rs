use std::time::Duration;

use async_trait::async_trait;
use fantoccini::{ClientBuilder, Locator};

use super::Source;

const ELEMENT_TIMEOUT: Duration = Duration::from_secs(40);

/// RC Deportivo ticketing monitor.
///
/// Navigates to the ticketing page, waits for a match card to render, then
/// checks whether the `.fa-user-lock` icon is present inside it.
///
/// Returns one of two fixed strings so the generic ChangeDetector can track
/// the transition and fire a notification exactly once per state change:
///
///   "🔒 locked"    — fa-user-lock is present (tickets not yet on sale)
///   "🔓 unlocked"  — fa-user-lock is gone   (tickets may be available)
pub struct RcDeportivoSource {
    url: String,
    /// CSS selector for the match card container, e.g. `[id="237"]`.
    container_selector: String,
    webdriver_url: String,
    location: String,
}

impl RcDeportivoSource {
    pub fn new(url: String, container_selector: String, webdriver_url: String) -> Self {
        let location = format!("{url}  [{container_selector}]");
        Self { url, container_selector, webdriver_url, location }
    }

    fn lock_selector(&self) -> String {
        format!("{} .fa-user-lock", self.container_selector)
    }
}

#[async_trait]
impl Source for RcDeportivoSource {
    fn location(&self) -> &str {
        &self.location
    }

    async fn fetch(&self) -> anyhow::Result<String> {
        let mut caps = serde_json::Map::new();
        caps.insert(
            "goog:chromeOptions".into(),
            serde_json::json!({
                "args": [
                    "--no-sandbox",
                    "--disable-dev-shm-usage",
                    "--disable-gpu",
                    "--headless=new"
                ]
            }),
        );

        let client = ClientBuilder::native()
            .capabilities(caps)
            .connect(&self.webdriver_url)
            .await
            .map_err(|e| anyhow::anyhow!(
                "Cannot connect to WebDriver at {} — is Chrome running? {e}",
                self.webdriver_url
            ))?;

        let result = self.check_lock(&client).await;
        let _ = client.close().await;
        result
    }
}

impl RcDeportivoSource {
    async fn check_lock(&self, client: &fantoccini::Client) -> anyhow::Result<String> {
        client
            .goto(&self.url)
            .await
            .map_err(|e| anyhow::anyhow!("Navigation to {} failed: {e}", self.url))?;

        // Wait for the match card to be rendered by Angular.
        client
            .wait()
            .at_most(ELEMENT_TIMEOUT)
            .for_element(Locator::Css(&self.container_selector))
            .await
            .map_err(|e| anyhow::anyhow!(
                "Match card '{}' did not appear within {:?}: {e}",
                self.container_selector,
                ELEMENT_TIMEOUT,
            ))?;

        // Boolean check: is the lock icon still in the DOM?
        let is_locked = client
            .execute(
                "return !!document.querySelector(arguments[0]);",
                vec![serde_json::json!(self.lock_selector())],
            )
            .await
            .map_err(|e| anyhow::anyhow!("JS presence check failed: {e}"))?
            .as_bool()
            .unwrap_or(false);

        if is_locked {
            Ok("🔒 locked".to_string())
        } else {
            Ok("🔓 unlocked".to_string())
        }
    }
}
