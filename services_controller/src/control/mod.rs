pub mod docker;

use async_trait::async_trait;

use crate::manager::ServiceStatus;

/// The port through which the domain controls a running service.
///
/// Each backend (Docker, systemd, a remote API, …) provides one concrete
/// implementation. The [`crate::manager::ServiceManager`] depends only on this
/// trait, never on a specific backend — Docker is just one adapter.
///
/// The `service` argument is the backend-specific name already resolved from
/// an alias (e.g. a Docker container name).
#[async_trait]
pub trait ServiceController: Send + Sync {
    async fn start(&self, service: &str) -> anyhow::Result<()>;
    async fn stop(&self, service: &str) -> anyhow::Result<()>;
    async fn restart(&self, service: &str) -> anyhow::Result<()>;
    async fn status(&self, service: &str) -> anyhow::Result<ServiceStatus>;
}
