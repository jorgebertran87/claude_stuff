use async_trait::async_trait;
use std::path::PathBuf;

use super::Source;

/// Reads a local file and returns its contents as a UTF-8 string.
pub struct FileSource {
    path: PathBuf,
    /// Pre-computed display string so `location()` is zero-cost.
    location: String,
}

impl FileSource {
    pub fn new(path: PathBuf) -> Self {
        let location = path.display().to_string();
        Self { path, location }
    }
}

#[async_trait]
impl Source for FileSource {
    fn location(&self) -> &str {
        &self.location
    }

    async fn fetch(&self) -> anyhow::Result<String> {
        tokio::fs::read_to_string(&self.path)
            .await
            .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {e}", self.path))
    }
}
