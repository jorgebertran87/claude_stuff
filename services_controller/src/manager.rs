use std::sync::Arc;

use crate::{control::ServiceController, registry::ServiceRegistry};

/// The observable state of a service.
///
/// Deliberately coarse — the controller only needs to know whether a service
/// is up or down to satisfy start/stop/status requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceStatus {
    Running,
    Stopped,
}

impl ServiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceStatus::Running => "running",
            ServiceStatus::Stopped => "stopped",
        }
    }
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Drives services addressed by their alias.
///
/// This is the domain core: it resolves an alias to the underlying service
/// name via the [`ServiceRegistry`], then delegates the actual control to a
/// [`ServiceController`] port. It knows nothing about Docker, systemd, or any
/// other backend — those live behind the port.
pub struct ServiceManager {
    registry: ServiceRegistry,
    controller: Arc<dyn ServiceController>,
}

impl ServiceManager {
    pub fn new(registry: ServiceRegistry, controller: Arc<dyn ServiceController>) -> Self {
        Self { registry, controller }
    }

    /// Start the service behind `alias`.
    pub async fn start(&self, alias: &str) -> anyhow::Result<()> {
        let service = self.registry.resolve(alias)?;
        self.controller.start(service).await
    }

    /// Stop the service behind `alias`.
    pub async fn stop(&self, alias: &str) -> anyhow::Result<()> {
        let service = self.registry.resolve(alias)?;
        self.controller.stop(service).await
    }

    /// Restart the service behind `alias`.
    pub async fn restart(&self, alias: &str) -> anyhow::Result<()> {
        let service = self.registry.resolve(alias)?;
        self.controller.restart(service).await
    }

    /// Report the current status of the service behind `alias`.
    pub async fn status(&self, alias: &str) -> anyhow::Result<ServiceStatus> {
        let service = self.registry.resolve(alias)?;
        self.controller.status(service).await
    }
}
