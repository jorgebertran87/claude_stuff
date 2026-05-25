use std::path::Path;

use serde::{Deserialize, Serialize};

/// Configuration for a single dynamically-added CSS selector monitor.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MonitorConfig {
    pub alias: String,
    pub selector: String,
    pub interval_secs: u64,
}

/// Persists the list of dynamic monitors to `/data/monitors.json`.
/// Loaded at startup so monitors survive container restarts.
pub struct MonitorStore {
    path: std::path::PathBuf,
    monitors: Vec<MonitorConfig>,
}

impl MonitorStore {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("monitors.json");
        let monitors = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path, monitors }
    }

    pub fn all(&self) -> &[MonitorConfig] {
        &self.monitors
    }

    pub fn add(&mut self, config: MonitorConfig) -> anyhow::Result<()> {
        self.monitors.push(config);
        self.save()
    }

    /// Remove the monitor with the given alias from the store.
    /// Returns `true` if it was found and removed.
    pub fn remove(&mut self, alias: &str) -> anyhow::Result<bool> {
        let before = self.monitors.len();
        self.monitors.retain(|m| m.alias != alias);
        if self.monitors.len() < before {
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.monitors)?;
        std::fs::write(&self.path, &json)
            .map_err(|e| anyhow::anyhow!("Cannot save monitors to {:?}: {e}", self.path))
    }
}
