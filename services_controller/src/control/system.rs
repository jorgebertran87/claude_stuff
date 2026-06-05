use async_trait::async_trait;
use tokio::process::Command;

use super::{CommandOutput, CommandRunner};

/// Production [`CommandRunner`] that shells out via `tokio::process`.
pub struct SystemCommandRunner;

#[async_trait]
impl CommandRunner for SystemCommandRunner {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let output = Command::new(program)
            .args(args)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("failed to run {program}: {e}"))?;

        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
