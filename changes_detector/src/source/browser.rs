use std::time::Duration;

use async_trait::async_trait;
use fantoccini::{ClientBuilder, Locator};

use super::Source;

/// How long to wait for the target element to appear after page load.
/// A JS-heavy SPA typically renders within a few seconds; 40 s is generous.
const ELEMENT_TIMEOUT: Duration = Duration::from_secs(40);

/// After the page body is present, how long to keep polling before declaring
/// the element "absent" in existence mode.  Must be shorter than
/// ELEMENT_TIMEOUT so we still have time to navigate.
const EXISTENCE_SETTLE: Duration = Duration::from_secs(25);

/// How the source interprets the CSS selector.
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserMode {
    /// Return the element's outerHTML (collapsed whitespace).
    /// Any HTML change triggers a notification.
    Content,
    /// Return `"present"` or `"absent"` depending on whether the selector
    /// matches any element.  A state flip triggers a notification.
    Existence,
}

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
    mode: BrowserMode,
}

impl BrowserSource {
    /// Create a source with an explicit mode.
    pub fn with_mode(
        url: String,
        selector: Option<String>,
        webdriver_url: String,
        mode: BrowserMode,
    ) -> Self {
        let location = match (&selector, &mode) {
            (Some(s), BrowserMode::Existence) => format!("{url}  [{s}] (existence)"),
            (Some(s), _)                      => format!("{url}  [{s}]"),
            _                                 => url.clone(),
        };
        Self { url, selector, webdriver_url, location, mode }
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
            // ── No selector: return all visible body text ──────────────────
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

            Some(selector) => match self.mode {
                // ── Content mode: wait for element, return outerHTML ───────
                BrowserMode::Content => {
                    // Wait until the element is present in the DOM.
                    client
                        .wait()
                        .at_most(ELEMENT_TIMEOUT)
                        .for_element(Locator::Css(selector.as_str()))
                        .await
                        .map_err(|e| anyhow::anyhow!(
                            "Timed out waiting for '{selector}' to appear \
                             (JS may still be loading — consider raising \
                             CHECK_INTERVAL_SECS): {e}"
                        ))?;

                    // Extract outerHTML with collapsed whitespace via JS.
                    // outerHTML captures attribute changes (e.g. icon classes).
                    let raw = client
                        .execute(
                            "var el = document.querySelector(arguments[0]); \
                             return el ? el.outerHTML.replace(/\\s+/g, ' ').trim() : null;",
                            vec![serde_json::json!(selector)],
                        )
                        .await
                        .map_err(|e| anyhow::anyhow!(
                            "JS execution failed for '{selector}': {e}"
                        ))?;

                    raw.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| anyhow::anyhow!(
                            "Element '{selector}' disappeared after wait"
                        ))
                }

                // ── Existence mode: return "present" or "absent" ───────────
                //
                // Strategy: wait for the page body so we know the SPA has at
                // least started rendering, then poll `querySelector` every
                // second until either the element appears (→ "present") or
                // EXISTENCE_SETTLE elapses (→ "absent").
                BrowserMode::Existence => {
                    // Ensure the page itself has loaded.
                    client
                        .wait()
                        .at_most(ELEMENT_TIMEOUT)
                        .for_element(Locator::Css("body"))
                        .await
                        .map_err(|e| anyhow::anyhow!("Timed out waiting for page to load: {e}"))?;

                    let deadline = tokio::time::Instant::now() + EXISTENCE_SETTLE;
                    loop {
                        let found = client
                            .execute(
                                "return document.querySelector(arguments[0]) !== null;",
                                vec![serde_json::json!(selector)],
                            )
                            .await
                            .map_err(|e| anyhow::anyhow!(
                                "JS existence check failed for '{selector}': {e}"
                            ))?
                            .as_bool()
                            .unwrap_or(false);

                        if found {
                            return Ok("present".to_string());
                        }
                        if tokio::time::Instant::now() >= deadline {
                            break;
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }

                    Ok("absent".to_string())
                }
            },
        }
    }
}
