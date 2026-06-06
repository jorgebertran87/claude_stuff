pub mod ssh;
pub mod system;

use async_trait::async_trait;

/// The outcome of running a command on the host.
///
/// A non-zero `exit_code` is a normal result (the command ran and failed) — it
/// is relayed to the chat, not treated as an internal error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Port: run a command on the host and get its output back.
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, command: &str) -> anyhow::Result<CommandOutput>;
}

/// Low-level port for spawning an external process.
///
/// Production uses [`system::SystemCommandRunner`] (real `tokio::process`);
/// tests inject a fake that records the argv and returns canned output. This is
/// what keeps the ssh adapter unit-testable without invoking real `ssh`.
#[async_trait]
pub trait CommandRunner: Send + Sync {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput>;
}
