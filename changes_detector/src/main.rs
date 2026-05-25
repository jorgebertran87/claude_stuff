mod config;
mod telegram;
mod watcher;

use config::Config;
use telegram::TelegramNotifier;
use tracing::{error, info, warn};
use watcher::{compute_diff, load_state, read_file, save_state};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load a .env file if present (convenient for local dev; ignored in prod).
    let _ = dotenvy::dotenv();

    // Structured logging — level controlled via RUST_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "changes_detector=info".into()),
        )
        .init();

    let cfg = Config::from_env()?;

    info!(
        file   = %cfg.monitor_file.display(),
        interval_secs = cfg.check_interval.as_secs(),
        "Changes detector starting"
    );

    let notifier = TelegramNotifier::new(
        cfg.telegram_bot_token.clone(),
        cfg.telegram_chat_id.clone(),
    );

    // -----------------------------------------------------------------------
    // Bootstrap: if no state exists yet, snapshot the current file content
    // so the first real cycle has something to diff against.
    // -----------------------------------------------------------------------
    let mut prev = load_state(&cfg.state_file);
    if prev.is_none() {
        info!("No previous state found — creating initial snapshot");
        match read_file(&cfg.monitor_file) {
            Ok(state) => {
                if let Err(e) = save_state(&cfg.state_file, &state) {
                    warn!("Could not persist initial state: {}", e);
                } else {
                    info!(hash = &state.hash[..8], "Initial snapshot saved");
                }
                prev = Some(state);
            }
            Err(e) => error!("Cannot read monitored file on startup: {}", e),
        }
    } else {
        info!("Loaded previous state from {:?}", cfg.state_file);
    }

    // -----------------------------------------------------------------------
    // Main polling loop
    // -----------------------------------------------------------------------
    let mut interval = tokio::time::interval(cfg.check_interval);
    // First tick fires immediately; skip it so we don't double-check on boot.
    interval.tick().await;

    loop {
        interval.tick().await;
        info!("Checking for changes…");

        let current = match read_file(&cfg.monitor_file) {
            Ok(s) => s,
            Err(e) => {
                warn!("Cannot read file: {}", e);
                continue;
            }
        };

        let changed = prev
            .as_ref()
            .map(|p| p.hash != current.hash)
            .unwrap_or(true);

        if !changed {
            info!("No changes detected");
            continue;
        }

        // Something changed — compute diff and notify.
        let diff = prev
            .as_ref()
            .map(|p| compute_diff(&p.content, &current.content, 3_000))
            .unwrap_or_else(|| "(first read — no previous content)".into());

        info!("Change detected — sending Telegram notification");

        let location = cfg.monitor_file.display().to_string();
        match notifier.send_change_notification(&location, &diff).await {
            Ok(_) => info!("Telegram notification sent"),
            Err(e) => error!("Failed to send Telegram notification: {}", e),
        }

        // Persist the new state so the next cycle diffs correctly.
        if let Err(e) = save_state(&cfg.state_file, &current) {
            error!("Failed to save new state: {}", e);
        }
        prev = Some(current);
    }
}
