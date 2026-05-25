mod config;
mod detector;
mod source;
mod telegram;

use config::Config;
use detector::{ChangeDetector, CheckResult};
use source::{file::FileSource, Source};
use telegram::TelegramNotifier;
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
    // This is the only place that knows about infrastructure.  To add a new
    // kind of source (HTTP endpoint, S3 object, database query, …) add a new
    // branch here and a new file under src/source/ — nothing else changes.
    // -----------------------------------------------------------------------
    let source: Box<dyn Source> = Box::new(FileSource::new(cfg.monitor_target.into()));

    info!(
        location       = source.location(),
        interval_secs  = cfg.check_interval.as_secs(),
        "Changes detector starting"
    );

    let notifier = TelegramNotifier::new(cfg.telegram_bot_token, cfg.telegram_chat_id);
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
