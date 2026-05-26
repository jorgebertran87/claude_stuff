mod config;
mod detector;
mod monitor;
mod runner;
mod source;
mod telegram;

use std::{collections::HashMap, sync::Arc};

use config::Config;
use monitor::MonitorStore;
use runner::MonitorSpawner;
use telegram::{CommandHandler, TelegramNotifier};
use tokio::sync::Mutex;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "changes_detector=info".into()),
        )
        .init();

    let cfg = Config::from_env()?;

    // -----------------------------------------------------------------------
    // Create the notifier and spawner.
    // -----------------------------------------------------------------------
    let notifier = Arc::new(TelegramNotifier::new(
        cfg.telegram_bot_token.clone(),
        cfg.telegram_chat_id.clone(),
    ));

    let spawner = MonitorSpawner {
        webdriver_url:    cfg.webdriver_url.clone(),
        flaresolverr_url: cfg.flaresolverr_url.clone(),
        notifier:         notifier.clone(),
        data_dir:         cfg.data_dir.clone(),
        tasks:            Arc::new(Mutex::new(HashMap::new())),
    };

    // -----------------------------------------------------------------------
    // Load the monitor store and resume any previously-added monitors.
    // -----------------------------------------------------------------------
    let store = {
        let loaded = MonitorStore::load(&cfg.data_dir);
        for monitor_cfg in loaded.all() {
            if monitor_cfg.paused {
                info!("Skipping paused monitor: {}", monitor_cfg.alias);
                continue;
            }
            info!("Resuming monitor: {} ({})", monitor_cfg.alias, monitor_cfg.selector);
            spawner.spawn(monitor_cfg.clone()).await;
        }
        Arc::new(Mutex::new(loaded))
    };

    info!("Changes detector starting — {} monitor(s) active", {
        store.lock().await.all().len()
    });

    // -----------------------------------------------------------------------
    // Spawn the Telegram command handler (runs until the process exits).
    // -----------------------------------------------------------------------
    let cmd_handler = CommandHandler::new(
        cfg.telegram_bot_token.clone(),
        &cfg.telegram_chat_id,
        spawner,
        store,
    )?;
    tokio::spawn(cmd_handler.run());

    // Keep the process alive until SIGINT / SIGTERM.
    tokio::signal::ctrl_c().await.ok();
    info!("Shutdown signal received — exiting");

    Ok(())
}
