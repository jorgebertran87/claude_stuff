pub mod compose;
pub mod system;

use async_trait::async_trait;

use crate::manager::ServiceStatus;

/// The port through which the domain controls a running service.
///
/// Each backend (docker compose, systemd, a remote API, …) provides one
/// concrete implementation. The [`crate::manager::ServiceManager`] depends only
/// on this trait — the backend is just an adapter.
///
/// The `service` argument is the backend-specific target already resolved from
/// an alias (for the compose adapter, the directory of a compose project).
#[async_trait]
pub trait ServiceController: Send + Sync {
    async fn start(&self, service: &str) -> anyhow::Result<()>;
    async fn stop(&self, service: &str) -> anyhow::Result<()>;
    async fn restart(&self, service: &str) -> anyhow::Result<()>;
    async fn status(&self, service: &str) -> anyhow::Result<ServiceStatus>;
}

/// The outcome of running an external command.
#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Port for executing external commands.
///
/// Production uses [`system::SystemCommandRunner`] (real `tokio::process`);
/// tests inject a fake that records the argv and returns canned output. This is
/// what keeps the compose adapter unit-testable without invoking real Docker.
#[async_trait]
pub trait CommandRunner: Send + Sync {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput>;
}
