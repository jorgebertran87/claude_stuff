use async_trait::async_trait;
use serde::Deserialize;

use super::ServiceController;
use crate::manager::ServiceStatus;

/// Controls services by driving the Docker Engine HTTP API.
///
/// `api_url` points at a reachable Docker daemon endpoint — typically a TCP
/// proxy in front of the socket (`http://localhost:2375`) or the service name
/// of such a proxy in docker-compose. Docker is just one adapter behind the
/// [`ServiceController`] port; nothing in the domain knows it is used.
pub struct DockerController {
    api_url: String,
    client: reqwest::Client,
}

impl DockerController {
    pub fn new(api_url: String) -> Self {
        Self { api_url, client: reqwest::Client::new() }
    }

    /// POST to a `/containers/{service}/{action}` control endpoint and treat
    /// 2xx (success) and 304 (already in the desired state) as success.
    async fn control(&self, service: &str, action: &str) -> anyhow::Result<()> {
        let url = format!("{}/containers/{service}/{action}", self.api_url);
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Docker request to {url} failed: {e}"))?;

        let status = resp.status();
        // 304 Not Modified == the container was already started/stopped.
        if status.is_success() || status.as_u16() == 304 {
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Docker {action} of \"{service}\" returned {status}: {body}");
        }
    }
}

// ---------------------------------------------------------------------------
// Docker Engine API inspect DTOs (GET /containers/{id}/json)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct InspectResponse {
    #[serde(rename = "State")]
    state: ContainerState,
}

#[derive(Deserialize)]
struct ContainerState {
    #[serde(rename = "Running")]
    running: bool,
}

// ---------------------------------------------------------------------------
// ServiceController implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ServiceController for DockerController {
    async fn start(&self, service: &str) -> anyhow::Result<()> {
        self.control(service, "start").await
    }

    async fn stop(&self, service: &str) -> anyhow::Result<()> {
        self.control(service, "stop").await
    }

    async fn restart(&self, service: &str) -> anyhow::Result<()> {
        self.control(service, "restart").await
    }

    async fn status(&self, service: &str) -> anyhow::Result<ServiceStatus> {
        let url = format!("{}/containers/{service}/json", self.api_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Docker request to {url} failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Docker inspect of \"{service}\" returned {status}: {body}");
        }

        let parsed: InspectResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Docker inspect response: {e}"))?;

        Ok(if parsed.state.running {
            ServiceStatus::Running
        } else {
            ServiceStatus::Stopped
        })
    }
}
