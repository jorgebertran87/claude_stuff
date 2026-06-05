use std::sync::Arc;

use anyhow::Context;

use services_controller::{
    config::Config,
    control::docker::DockerController,
    manager::ServiceManager,
    registry::ServiceRegistry,
    telegram::{http::HttpTelegramGateway, TelegramBot, TelegramGateway},
};

/// Usage:
///   services_controller <start|stop|restart|status> <alias>   — one-shot CLI
///   services_controller bot                                    — Telegram bot
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;

    // Wire the hexagon: domain manager + Docker adapter behind the port.
    let registry = ServiceRegistry::load(&config.alias_config)?;
    let controller = Arc::new(DockerController::new(config.docker_api_url));
    let manager = Arc::new(ServiceManager::new(registry, controller));

    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .context("usage: services_controller <start|stop|restart|status> <alias> | bot")?;

    if command == "bot" {
        let token = config
            .telegram_bot_token
            .context("TELEGRAM_BOT_TOKEN is required for bot mode")?;
        let gateway: Arc<dyn TelegramGateway> = Arc::new(HttpTelegramGateway::new(token));
        TelegramBot::new(gateway, manager, config.telegram_allowed_chats)
            .run()
            .await;
        return Ok(());
    }

    let alias = args.next().context("missing alias argument")?;
    match command.as_str() {
        "start" => {
            manager.start(&alias).await?;
            println!("started {alias}");
        }
        "stop" => {
            manager.stop(&alias).await?;
            println!("stopped {alias}");
        }
        "restart" => {
            manager.restart(&alias).await?;
            println!("restarted {alias}");
        }
        "status" => {
            let status = manager.status(&alias).await?;
            println!("{alias}: {status}");
        }
        other => anyhow::bail!("unknown command \"{other}\" (expected start|stop|restart|status|bot)"),
    }

    Ok(())
}
