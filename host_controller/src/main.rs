//! Entry point: build the configuration and adapters, then run the bot.
//!
//! Wiring only — the behaviour lives in the library and is tested there.

use std::sync::Arc;

use host_controller::{
    authorizer::Authorizer,
    config::Config,
    executor::{ssh::SshExecutor, system::SystemCommandRunner, CommandExecutor},
    telegram::{http::HttpTelegramGateway, TelegramBot, TelegramGateway},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;

    tracing::info!(
        "host_controller starting: ssh {}@{}:{}, {} allowed chat(s), {}s command timeout",
        config.ssh_user,
        config.ssh_host,
        config.ssh_port,
        config.allowed_chats.len(),
        config.command_timeout.as_secs(),
    );
    if config.allowed_chats.is_empty() {
        tracing::warn!(
            "TELEGRAM_ALLOWED_CHATS is empty — the bot will ignore every message (deny-all)"
        );
    }

    // Wire the hexagon: the bot depends only on the ports.
    let executor: Arc<dyn CommandExecutor> = Arc::new(SshExecutor::new(
        Arc::new(SystemCommandRunner),
        config.ssh_user,
        config.ssh_host,
        config.ssh_port,
        config.ssh_key,
        config.ssh_known_hosts,
    ));
    let gateway: Arc<dyn TelegramGateway> = Arc::new(HttpTelegramGateway::new(config.bot_token));
    let authorizer = Authorizer::new(config.allowed_chats);

    TelegramBot::new(gateway, authorizer, executor, config.command_timeout)
        .run()
        .await;

    Ok(())
}
