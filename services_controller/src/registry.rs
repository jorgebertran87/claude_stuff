use std::{collections::HashMap, path::Path};

/// Maps stable, human-friendly aliases to the target each backend understands.
///
/// For the docker compose backend the target is the directory holding the
/// service's `docker-compose.yml`. Declared in a YAML config file:
///
/// ```yaml
/// web: /srv/web
/// db: /srv/db
/// ```
pub struct ServiceRegistry {
    aliases: HashMap<String, String>,
}

impl ServiceRegistry {
    /// Build a registry from an in-memory alias → service map.
    pub fn from_map(aliases: HashMap<String, String>) -> Self {
        Self { aliases }
    }

    /// Load the alias map from a YAML config file on disk.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Cannot read alias config {path:?}: {e}"))?;
        let aliases: HashMap<String, String> = serde_yaml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Invalid alias config {path:?}: {e}"))?;
        Ok(Self { aliases })
    }

    /// Resolve an alias to its underlying service name.
    /// Returns an "unknown alias" error if the alias is not declared.
    pub fn resolve(&self, alias: &str) -> anyhow::Result<&str> {
        self.aliases
            .get(alias)
            .map(String::as_str)
            .ok_or_else(|| anyhow::anyhow!("unknown alias \"{alias}\""))
    }
}
