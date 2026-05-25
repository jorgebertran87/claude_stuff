mod config;
mod detector;
mod source;
mod telegram;

use config::Config;
use detector::{ChangeDetector, CheckResult};
use source::{browser::BrowserSource, file::FileSource, http::HttpSource, Source};
use telegram::{CommandHandler, TelegramNotifier};
use tracing::{error, info, warn};

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
    // Wire the source.
    //
    // SOURCE_TYPE selects explicitly; when absent the type is inferred:
    //   "browser"                          → BrowserSource (headless Chrome)
    //   "http" | http(s):// prefix         → HttpSource    (plain HTTP fetch)
    //   "file" | anything else             → FileSource
    //
    // To add a new source: create src/source/<name>.rs, implement the trait,
    // add a branch here — nothing else changes.
    // -----------------------------------------------------------------------
    let is_url = cfg.monitor_target.starts_with("http://")
        || cfg.monitor_target.starts_with("https://");

    let source: Box<dyn Source> = match cfg.source_type.as_deref() {
        Some("browser") => Box::new(BrowserSource::new(
            cfg.monitor_target,
            cfg.html_selector,
            cfg.webdriver_url,
        )),
        Some("http") => Box::new(HttpSource::new(
            cfg.monitor_target,
            cfg.html_selector.as_deref(),
        )?),
        Some("file") => Box::new(FileSource::new(cfg.monitor_target.into())),
        // Auto-detect
        _ if is_url => Box::new(HttpSource::new(
            cfg.monitor_target,
            cfg.html_selector.as_deref(),
        )?),
        _ => Box::new(FileSource::new(cfg.monitor_target.into())),
    };

    info!(
        location       = source.location(),
        interval_secs  = cfg.check_interval.as_secs(),
        "Changes detector starting"
    );

    let notifier = TelegramNotifier::new(
        cfg.telegram_bot_token.clone(),
        cfg.telegram_chat_id.clone(),
    );

    // Spawn the command handler in the background so /status and /check
    // can be sent to the bot at any time without blocking the polling loop.
    let cmd_handler = CommandHandler::new(
        cfg.telegram_bot_token,
        &cfg.telegram_chat_id,
        source.location().to_string(),
        cfg.check_interval.as_secs(),
    )?;
    tokio::spawn(cmd_handler.run());
    let mut detector = ChangeDetector::load(&cfg.state_file);

    // -----------------------------------------------------------------------
    // Bootstrap: take an initial snapshot on startup so the first real cycle
    // always has a previous state to diff against.
    // -----------------------------------------------------------------------
    match source.fetch().await {
        Ok(content) => match detector.check(content)? {
            CheckResult::Bootstrapped => info!("Initial snapshot saved"),
            CheckResult::NoChange    => info!("Resumed — content unchanged since last run"),
            CheckResult::Changed { .. } => info!("State updated on startup (file changed while stopped)"),
        },
        Err(e) => error!("Cannot fetch source on startup: {e}"),
    }

    // -----------------------------------------------------------------------
    // Main polling loop — completely source-agnostic.
    // -----------------------------------------------------------------------
    let mut interval = tokio::time::interval(cfg.check_interval);
    interval.tick().await; // first tick fires immediately; skip it

    loop {
        interval.tick().await;
        info!("Checking for changes…");

        let content = match source.fetch().await {
            Ok(c)  => c,
            Err(e) => { warn!("Fetch failed: {e}"); continue; }
        };

        match detector.check(content)? {
            CheckResult::NoChange       => info!("No changes"),
            CheckResult::Bootstrapped   => info!("Snapshot saved (first run)"),
            CheckResult::Changed { diff } => {
                info!("Change detected — sending notification");
                if let Err(e) = notifier.send_change_notification(source.location(), &diff).await {
                    error!("Telegram notification failed: {e}");
                }
            }
        }
    }
}
