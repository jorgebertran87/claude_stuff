use std::sync::Arc;

use async_trait::async_trait;

use super::{CommandExecutor, CommandOutput, CommandRunner};

/// Runs commands on the parent host by shelling out to `ssh` through the
/// [`CommandRunner`] port. The session is non-interactive (`BatchMode=yes`, so
/// it never hangs on a prompt) and host-key verification is enforced
/// (`StrictHostKeyChecking=yes` against a pinned `known_hosts`), so an unknown
/// or swapped host key is refused.
///
/// Running through the port keeps the argument building unit-testable with a
/// fake runner — real `ssh` is exercised only by the integration suite.
pub struct SshExecutor {
    runner: Arc<dyn CommandRunner>,
    user: String,
    host: String,
    port: u16,
    key: String,
    known_hosts: String,
}

impl SshExecutor {
    pub fn new(
        runner: Arc<dyn CommandRunner>,
        user: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        key: impl Into<String>,
        known_hosts: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            user: user.into(),
            host: host.into(),
            port,
            key: key.into(),
            known_hosts: known_hosts.into(),
        }
    }

    /// Build the `ssh` argv that runs `command` on the host.
    fn args(&self, command: &str) -> Vec<String> {
        vec![
            "-i".to_string(),
            self.key.clone(),
            "-p".to_string(),
            self.port.to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            "-o".to_string(),
            format!("UserKnownHostsFile={}", self.known_hosts),
            format!("{}@{}", self.user, self.host),
            command.to_string(),
        ]
    }
}

#[async_trait]
impl CommandExecutor for SshExecutor {
    async fn execute(&self, command: &str) -> anyhow::Result<CommandOutput> {
        self.runner.run("ssh", &self.args(command)).await
    }
}
