use std::{path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use tracing::{error, info, warn};

use crate::{
    detector::{ChangeDetector, CheckResult},
    monitor::MonitorConfig,
    source::{browser::BrowserSource, Source},
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
    pub base_url: String,
    pub webdriver_url: String,
    pub notifier: Arc<dyn Notifier>,
    pub data_dir: PathBuf,
}

impl MonitorSpawner {
    /// Spawn a `tokio` task that runs `run_loop` for the given config.
    pub fn spawn(&self, config: MonitorConfig) {
        let s = self.clone();
        tokio::spawn(async move {
            let source: Arc<dyn Source> = Arc::new(BrowserSource::new(
                s.base_url.clone(),
                Some(config.selector.clone()),
                s.webdriver_url.clone(),
            ));
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
