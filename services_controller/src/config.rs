use std::path::PathBuf;

pub struct Config {
    /// Path to the YAML file declaring alias → service mappings.
    /// Default: `/config/aliases.yaml` (mount it there in the container).
    pub alias_config: PathBuf,
    /// Docker Engine HTTP API endpoint used by `DockerController`.
    /// Default: `http://localhost:2375` (a TCP proxy in front of the socket).
    pub docker_api_url: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let alias_config = std::env::var("ALIAS_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/config/aliases.yaml"));

        let docker_api_url = std::env::var("DOCKER_API_URL")
            .unwrap_or_else(|_| "http://localhost:2375".into());

        Ok(Self { alias_config, docker_api_url })
    }
}
