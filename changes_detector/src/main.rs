mod config;
mod detector;
mod monitor;
mod runner;
mod source;
mod telegram;

use std::sync::Arc;

use config::Config;
use detector::ChangeDetector;
use monitor::MonitorStore;
use runner::{MonitorSpawner, run_loop};
use source::{browser::BrowserSource, file::FileSource, http::HttpSource, Source};
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
    // Build the primary source.
    // -----------------------------------------------------------------------
    let is_url = cfg.monitor_target.starts_with("http://")
        || cfg.monitor_target.starts_with("https://");

    let source: Arc<dyn Source> = match cfg.source_type.as_deref() {
        Some("browser") => Arc::new(BrowserSource::new(
            cfg.monitor_target.clone(),
            cfg.html_selector.clone(),
            cfg.webdriver_url.clone(),
        )),
        Some("http") => Arc::new(HttpSource::new(
            cfg.monitor_target.clone(),
            cfg.html_selector.as_deref(),
        )?),
        Some("file") => Arc::new(FileSource::new(cfg.monitor_target.clone().into())),
        _ if is_url => Arc::new(HttpSource::new(
            cfg.monitor_target.clone(),
            cfg.html_selector.as_deref(),
        )?),
        _ => Arc::new(FileSource::new(cfg.monitor_target.clone().into())),
    };

    info!(
        location      = source.location(),
        interval_secs = cfg.check_interval.as_secs(),
        "Changes detector starting"
    );

    // -----------------------------------------------------------------------
    // Create the notifier and spawner.
    // -----------------------------------------------------------------------
    let notifier = Arc::new(TelegramNotifier::new(
        cfg.telegram_bot_token.clone(),
        cfg.telegram_chat_id.clone(),
    ));

    let spawner = MonitorSpawner {
        base_url:     cfg.monitor_target.clone(),
        webdriver_url: cfg.webdriver_url.clone(),
        notifier:     notifier.clone(),
        data_dir:     cfg.data_dir.clone(),
    };

    // -----------------------------------------------------------------------
    // Load the monitor store and resume any previously-added monitors.
    // -----------------------------------------------------------------------
    let store = {
        let loaded = MonitorStore::load(&cfg.data_dir);
        for config in loaded.all() {
            info!("Resuming monitor: {} ({})", config.alias, config.selector);
            spawner.spawn(config.clone());
        }
        Arc::new(Mutex::new(loaded))
    };

    // -----------------------------------------------------------------------
    // Spawn the Telegram command handler.
    // -----------------------------------------------------------------------
    let cmd_handler = CommandHandler::new(
        cfg.telegram_bot_token.clone(),
        &cfg.telegram_chat_id,
        Arc::clone(&source),
        cfg.check_interval.as_secs(),
        spawner,
        store,
    )?;
    tokio::spawn(cmd_handler.run());

    // -----------------------------------------------------------------------
    // Run the primary monitoring loop (blocks until the process is killed).
    // -----------------------------------------------------------------------
    let detector = ChangeDetector::load(&cfg.state_file);
    run_loop(source, detector, notifier, "main".to_string(), cfg.check_interval).await;

    Ok(())
}
