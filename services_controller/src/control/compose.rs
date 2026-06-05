use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use super::{CommandRunner, ServiceController};
use crate::manager::ServiceStatus;

/// Controls a service by driving its `docker compose` project.
///
/// The resolved `service` is the directory holding the project's
/// `docker-compose.yml`. start/stop/restart map straight to the matching
/// compose subcommand; status reads `docker compose ps --format json`.
///
/// Execution goes through the [`CommandRunner`] port, so the command building
/// and `ps` parsing are unit-tested with a fake runner — Docker stays an
/// implementation detail.
pub struct ComposeController {
    runner: Arc<dyn CommandRunner>,
}

impl ComposeController {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }

    /// `<dir>/docker-compose.yml`, tolerating a trailing slash on the dir.
    fn compose_file(dir: &str) -> String {
        format!("{}/docker-compose.yml", dir.trim_end_matches('/'))
    }

    /// Run `docker compose -f <dir>/docker-compose.yml <extra…>`.
    async fn compose(&self, dir: &str, extra: &[&str]) -> anyhow::Result<super::CommandOutput> {
        let mut args = vec![
            "compose".to_string(),
            "-f".to_string(),
            Self::compose_file(dir),
        ];
        args.extend(extra.iter().map(|s| s.to_string()));
        self.runner.run("docker", &args).await
    }

    /// Run a state-changing compose subcommand, failing on a non-zero exit.
    async fn action(&self, dir: &str, action: &str) -> anyhow::Result<()> {
        let out = self.compose(dir, &[action]).await?;
        if out.success {
            Ok(())
        } else {
            anyhow::bail!("docker compose {action} in {dir} failed: {}", out.stderr.trim());
        }
    }
}

/// One entry of `docker compose ps --format json`.
#[derive(Deserialize)]
struct PsEntry {
    #[serde(rename = "State")]
    state: String,
}

impl PsEntry {
    fn is_running(&self) -> bool {
        self.state == "running"
    }
}

/// Parse `docker compose ps --format json`, which is either a JSON array
/// (newer Compose) or newline-delimited JSON objects (older Compose).
/// A service is considered running if any container reports state "running".
fn any_running(stdout: &str) -> bool {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return false;
    }
    if let Ok(entries) = serde_json::from_str::<Vec<PsEntry>>(trimmed) {
        return entries.iter().any(PsEntry::is_running);
    }
    trimmed
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<PsEntry>(l).ok())
        .any(|e| e.is_running())
}

#[async_trait]
impl ServiceController for ComposeController {
    async fn start(&self, service: &str) -> anyhow::Result<()> {
        self.action(service, "start").await
    }

    async fn stop(&self, service: &str) -> anyhow::Result<()> {
        self.action(service, "stop").await
    }

    async fn restart(&self, service: &str) -> anyhow::Result<()> {
        self.action(service, "restart").await
    }

    async fn status(&self, service: &str) -> anyhow::Result<ServiceStatus> {
        let out = self.compose(service, &["ps", "--format", "json"]).await?;
        if !out.success {
            anyhow::bail!("docker compose ps in {service} failed: {}", out.stderr.trim());
        }
        Ok(if any_running(&out.stdout) {
            ServiceStatus::Running
        } else {
            ServiceStatus::Stopped
        })
    }
}
