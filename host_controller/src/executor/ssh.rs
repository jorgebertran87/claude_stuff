use std::sync::Arc;

use async_trait::async_trait;

use super::{CommandExecutor, CommandOutput, CommandRunner};

/// Runs commands on the parent host by shelling out to `ssh` through the
/// [`CommandRunner`] port. The session is non-interactive (`BatchMode=yes`, so
/// it never hangs on a prompt) and host-key verification is enforced
/// (`StrictHostKeyChecking=yes` against a pinned `known_hosts`), so an unknown
/// or swapped host key is refused.
///
/// Where and how to connect: the ssh target plus the credentials pinning it.
#[derive(Clone, Debug)]
pub struct SshConfig {
    pub user: String,
    pub host: String,
    pub port: u16,
    pub key: String,
    pub known_hosts: String,
}

/// Running through the port keeps the argument building unit-testable with a
/// fake runner — real `ssh` is exercised only by the integration suite.
pub struct SshExecutor {
    runner: Arc<dyn CommandRunner>,
    config: SshConfig,
}

impl SshExecutor {
    pub fn new(runner: Arc<dyn CommandRunner>, config: SshConfig) -> Self {
        Self { runner, config }
    }

    /// Build the `ssh` argv that runs `command` on the host.
    fn args(&self, command: &str) -> Vec<String> {
        vec![
            "-i".to_string(),
            self.config.key.clone(),
            "-p".to_string(),
            self.config.port.to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            "-o".to_string(),
            format!("UserKnownHostsFile={}", self.config.known_hosts),
            format!("{}@{}", self.config.user, self.config.host),
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
