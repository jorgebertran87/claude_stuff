use std::sync::Arc;

use anyhow::Context;

use services_controller::{
    config::Config,
    control::docker::DockerController,
    manager::ServiceManager,
    registry::ServiceRegistry,
};

/// Usage:  services_controller <start|stop|restart|status> <alias>
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;

    // Wire the hexagon: domain manager + Docker adapter behind the port.
    let registry = ServiceRegistry::load(&config.alias_config)?;
    let controller = Arc::new(DockerController::new(config.docker_api_url));
    let manager = ServiceManager::new(registry, controller);

    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .context("usage: services_controller <start|stop|restart|status> <alias>")?;
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
        other => anyhow::bail!("unknown command \"{other}\" (expected start|stop|restart|status)"),
    }

    Ok(())
}
