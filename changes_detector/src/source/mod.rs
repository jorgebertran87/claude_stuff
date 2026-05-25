pub mod file;
pub mod http;

use async_trait::async_trait;

/// Anything that can produce a string to be monitored for changes.
///
/// Infrastructure concerns (filesystem I/O, HTTP, databases, …) live in
/// concrete implementations of this trait. The change-detection logic never
/// needs to know how the string was obtained.
#[async_trait]
pub trait Source: Send + Sync {
    /// Human-readable identifier shown in Telegram notifications
    /// (e.g. a file path, a URL, …).
    fn location(&self) -> &str;

    /// Fetch the current content. Called once per polling cycle.
    async fn fetch(&self) -> anyhow::Result<String>;
}
