use std::path::{Path, PathBuf};

/// Holds the current Glovo bearer token in a single file under `/data`.
///
/// The file is the only channel between the traffic capturer and the bot:
/// the mitmproxy addon writes a freshly captured token, and the bot reads
/// it back on the next request. `current` re-reads the file every time so
/// an out-of-band write is picked up without a restart.
pub struct TokenStore {
    path: PathBuf,
}

impl TokenStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The token currently on disk, or `None` when the file is missing,
    /// empty, or only whitespace.
    pub fn current(&self) -> Option<String> {
        let token = std::fs::read_to_string(&self.path).ok()?;
        let token = token.trim();
        if token.is_empty() {
            None
        } else {
            Some(token.to_string())
        }
    }

    /// Persist a token, replacing any previous one. Written atomically
    /// (temp file + rename) so a concurrent reader never sees a half-write.
    pub fn set(&self, token: &str) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, token.trim())?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

impl AsRef<Path> for TokenStore {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
