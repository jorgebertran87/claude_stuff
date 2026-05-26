use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::{
    detector::{ChangeDetector, CheckResult},
    monitor::{MonitorConfig, MonitorMode, SourceType},
    source::{
        browser::{BrowserMode, BrowserSource},
        flare::{FetchMode, FlareSolverSource},
        Source,
    },
};

// ---------------------------------------------------------------------------
// Notifier trait — decouples the loop from the Telegram implementation
// so runner.rs does not import telegram.rs (avoids circular dependency).
// ---------------------------------------------------------------------------

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, location: &str, diff: &str) -> anyhow::Result<()>;
}

// ---------------------------------------------------------------------------
// MonitorSpawner — creates independent monitoring tasks at runtime
// ---------------------------------------------------------------------------

/// Carries enough context to spawn a new monitoring task for any selector.
/// Cheap to clone — all heavy state is behind `Arc`.
#[derive(Clone)]
pub struct MonitorSpawner {
    pub webdriver_url: String,
    pub flaresolverr_url: String,
    pub notifier: Arc<dyn Notifier>,
    pub data_dir: PathBuf,
    /// Abort handles for every running monitor task, keyed by alias.
    pub tasks: Arc<Mutex<HashMap<String, tokio::task::AbortHandle>>>,
}

impl MonitorSpawner {
    /// Spawn a `tokio` task that runs `run_loop` for the given config and
    /// register its abort handle under `config.alias`.
    pub async fn spawn(&self, config: MonitorConfig) {
        let s = self.clone();
        let alias = config.alias.clone();
        let url = match config.url.clone() {
            Some(u) => u,
            None => {
                warn!(
                    "[{}] Skipping monitor — no URL configured \
                     (legacy entry without a url field in monitors.json). \
                     Remove it and re-add via /add.",
                    config.alias
                );
                return;
            }
        };

        let handle = tokio::spawn(async move {
            let source: Arc<dyn Source> = match config.source_type {
                SourceType::Browser => {
                    let mode = match config.mode {
                        MonitorMode::Content   => BrowserMode::Content,
                        MonitorMode::Existence => BrowserMode::Existence,
                    };
                    Arc::new(BrowserSource::with_mode(
                        url,
                        Some(config.selector.clone()),
                        s.webdriver_url.clone(),
                        mode,
                    ))
                }
                SourceType::Flare => {
                    let mode = match config.mode {
                        MonitorMode::Content   => FetchMode::Content,
                        MonitorMode::Existence => FetchMode::Existence,
                    };
                    Arc::new(FlareSolverSource::new(
                        url,
                        Some(config.selector.clone()),
                        mode,
                        s.flaresolverr_url.clone(),
                    ))
                }
            };
            let state_file = s.data_dir.join(format!("{}.state", config.alias));
            let detector = ChangeDetector::load(&state_file);
            run_loop(
                source,
                detector,
                s.notifier.clone(),
                config.alias,
                Duration::from_secs(config.interval_secs),
            )
            .await;
        });
        self.tasks.lock().await.insert(alias, handle.abort_handle());
    }

    /// Fetch the current text content of `config`'s selector using the
    /// appropriate source backend.  Returns plain text — HTML tags stripped.
    pub async fn fetch_text(&self, config: &MonitorConfig) -> anyhow::Result<String> {
        let url = config
            .url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Monitor has no URL configured"))?;

        let source: Arc<dyn Source> = match config.source_type {
            SourceType::Browser => Arc::new(BrowserSource::with_mode(
                url,
                Some(config.selector.clone()),
                self.webdriver_url.clone(),
                BrowserMode::Content,
            )),
            SourceType::Flare => Arc::new(FlareSolverSource::new(
                url,
                Some(config.selector.clone()),
                FetchMode::Content,
                self.flaresolverr_url.clone(),
            )),
        };

        let html = source.fetch().await?;
        Ok(strip_tags(&html))
    }

    /// Abort the running task for `alias` without removing it from the store.
    /// Returns `true` if a task was found and aborted.
    /// (The caller is responsible for persisting the paused state in MonitorStore.)
    pub async fn pause(&self, alias: &str) -> bool {
        if let Some(handle) = self.tasks.lock().await.remove(alias) {
            handle.abort();
            true
        } else {
            false
        }
    }

    /// Spawn a fresh task for a previously-paused monitor config.
    /// Equivalent to `spawn()` — provided as a named alias for clarity.
    pub async fn resume(&self, config: MonitorConfig) {
        self.spawn(config).await;
    }

    /// Abort the task for `alias` and remove it from the tracker.
    /// Returns `true` if a task was found and aborted.
    pub async fn remove(&self, alias: &str) -> bool {
        if let Some(handle) = self.tasks.lock().await.remove(alias) {
            handle.abort();
            true
        } else {
            false
        }
    }

    /// Return a sorted list of all currently-tracked monitor aliases.
    pub async fn list_aliases(&self) -> Vec<String> {
        let mut aliases: Vec<String> = self.tasks.lock().await.keys().cloned().collect();
        aliases.sort();
        aliases
    }
}

// ---------------------------------------------------------------------------
// run_loop — shared monitoring loop used by both the primary monitor and
//            every dynamically-added one
// ---------------------------------------------------------------------------

pub async fn run_loop(
    source: Arc<dyn Source>,
    mut detector: ChangeDetector,
    notifier: Arc<dyn Notifier>,
    alias: String,
    interval: Duration,
) {
    info!("[{alias}] Monitor starting — {}", source.location());

    // Bootstrap: take an initial snapshot.
    match source.fetch().await {
        Ok(content) => match detector.check(content) {
            Ok(CheckResult::Bootstrapped) => info!("[{alias}] Initial snapshot saved"),
            Ok(CheckResult::NoChange)     => info!("[{alias}] Resumed — no changes"),
            Ok(CheckResult::Changed { .. }) => info!("[{alias}] State updated on startup"),
            Err(e) => error!("[{alias}] Detector error on startup: {e}"),
        },
        Err(e) => error!("[{alias}] Initial fetch failed: {e}"),
    }

    let mut ticker = tokio::time::interval(interval);
    ticker.tick().await; // skip immediate first tick

    loop {
        ticker.tick().await;
        info!("[{alias}] Checking…");

        let content = match source.fetch().await {
            Ok(c)  => c,
            Err(e) => { warn!("[{alias}] Fetch failed: {e}"); continue; }
        };

        match detector.check(content) {
            Ok(CheckResult::Changed { diff }) => {
                info!("[{alias}] Change detected — notifying");
                if let Err(e) = notifier.notify(source.location(), &diff).await {
                    error!("[{alias}] Notification failed: {e}");
                }
            }
            Ok(CheckResult::NoChange)     => info!("[{alias}] No changes"),
            Ok(CheckResult::Bootstrapped) => {}
            Err(e) => error!("[{alias}] Detector error: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip HTML tags from a string and collapse whitespace.
/// Used to extract human-readable text from outerHTML for the /check command.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    // Decode the most common HTML entities and collapse whitespace.
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
