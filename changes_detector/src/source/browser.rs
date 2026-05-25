use std::time::Duration;

use async_trait::async_trait;
use fantoccini::{ClientBuilder, Locator};

use super::Source;

/// How long to wait for the target element to appear after page load.
/// A JS-heavy SPA typically renders within a few seconds; 30 s is generous.
const ELEMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Fetches a URL using a real headless browser (via WebDriver), waits for
/// JavaScript to render the page, then returns either the full visible body
/// text or the text content of a CSS-selected element.
///
/// Requires a running WebDriver server pointed to by `webdriver_url`
/// (e.g. `http://chrome:4444` for the `selenium/standalone-chrome` container).
///
/// A new browser session is opened and closed on every `fetch()` call so the
/// service is stateless across polling cycles and does not leak sessions.
pub struct BrowserSource {
    url: String,
    selector: Option<String>,
    webdriver_url: String,
    location: String,
}

impl BrowserSource {
    pub fn new(url: String, selector: Option<String>, webdriver_url: String) -> Self {
        let location = match &selector {
            Some(s) => format!("{url}  [{s}]"),
            None => url.clone(),
        };
        Self { url, selector, webdriver_url, location }
    }
}

#[async_trait]
impl Source for BrowserSource {
    fn location(&self) -> &str {
        &self.location
    }

    async fn fetch(&self) -> anyhow::Result<String> {
        // Chrome requires --no-sandbox when running as root inside a container.
        // --disable-dev-shm-usage avoids crashes caused by the limited /dev/shm
        // size that some container runtimes enforce even when shm_size is set.
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
                "Cannot connect to WebDriver at {} — is the Chrome container running? {e}",
                self.webdriver_url
            ))?;

        // Always close the browser session, even when extraction fails.
        let result = self.extract(&client).await;
        let _ = client.close().await;
        result
    }
}

impl BrowserSource {
    async fn extract(&self, client: &fantoccini::Client) -> anyhow::Result<String> {
        client
            .goto(&self.url)
            .await
            .map_err(|e| anyhow::anyhow!("Navigation to {} failed: {e}", self.url))?;

        match &self.selector {
            // No selector — wait for body and return all visible text.
            None => {
                client
                    .wait()
                    .at_most(ELEMENT_TIMEOUT)
                    .for_element(Locator::Css("body"))
                    .await
                    .map_err(|e| anyhow::anyhow!("Timed out waiting for page body: {e}"))?
                    .text()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read page body text: {e}"))
            }

            // Selector present — wait for the JS-rendered element, then build
            // a focused semantic snapshot:
            //   line 1: visible text (collapsed whitespace)
            //   line 2: fa-* icon class names present inside the element
            //            (omitted entirely when no icons are found)
            //
            // Using this representation instead of raw innerHTML means that
            // only meaningful changes — text or icon presence — trigger a
            // notification. Irrelevant HTML noise (attribute ordering,
            // whitespace, extra wrappers) is ignored.
            Some(selector) => {
                client
                    .wait()
                    .at_most(ELEMENT_TIMEOUT)
                    .for_element(Locator::Css(selector.as_str()))
                    .await
                    .map_err(|e| anyhow::anyhow!(
                        "Timed out waiting for '{selector}' \
                         (JS may still be loading): {e}"
                    ))?;

                let raw = client
                    .execute(
                        "(function(sel) { \
                            var el = document.querySelector(sel); \
                            if (!el) return null; \
                            var text = (el.innerText || el.textContent || '') \
                                           .replace(/\\s+/g, ' ').trim(); \
                            var icons = Array.from(el.querySelectorAll('[class]')) \
                                .flatMap(function(e) { return Array.from(e.classList); }) \
                                .filter(function(c) { return c.indexOf('fa-') === 0; }) \
                                .filter(function(c, i, a) { return a.indexOf(c) === i; }); \
                            return icons.length \
                                ? text + '\\n[icons: ' + icons.join(' ') + ']' \
                                : text; \
                        })(arguments[0])",
                        vec![serde_json::json!(selector)],
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("JS extraction failed for '{selector}': {e}"))?;

                raw.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| anyhow::anyhow!("Element '{selector}' not found"))
            }
        }
    }
}
