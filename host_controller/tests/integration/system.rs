use std::time::Duration;

use cucumber::{then, when, World};
use host_controller::executor::{system::SystemCommandRunner, CommandOutput, CommandRunner};
use tempfile::NamedTempFile;

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct SystemWorld {
    output: Option<CommandOutput>,
    abandoned_pid: Option<u32>,
    // Keeps the pid file alive for the duration of the scenario.
    pid_file: Option<NamedTempFile>,
}

async fn run_sh(script: String) -> anyhow::Result<CommandOutput> {
    SystemCommandRunner.run("sh", &["-c".to_string(), script]).await
}

/// Poll until the spawned command has written its pid to `path`.
async fn wait_for_pid(path: &std::path::Path) -> u32 {
    loop {
        if let Some(pid) = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            return pid;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// True while the process exists and is not a zombie. A killed child shows as
/// zombie ('Z') in /proc until the runtime reaps it, which counts as dead here.
fn process_alive(pid: u32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        // The state is the first field after the parenthesised command name.
        Ok(stat) => stat
            .rsplit(')')
            .next()
            .and_then(|rest| rest.trim_start().chars().next())
            .is_some_and(|state| state != 'Z'),
        Err(_) => false,
    }
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the runner runs a command printing "(.*)" that exits with code (\d+)$"#)]
async fn when_run(world: &mut SystemWorld, text: String, code: i32) {
    let output = run_sh(format!("echo {text}; exit {code}"))
        .await
        .expect("command failed to run");
    world.output = Some(output);
}

#[when("the runner abandons a long-running command")]
async fn when_abandon(world: &mut SystemWorld) {
    let pid_file = NamedTempFile::new().expect("failed to create pid file");
    let path = pid_file.path().to_owned();

    // `exec` makes sleep replace sh, so the written pid is the spawned pid.
    let mut run = Box::pin(run_sh(format!(
        "echo $$ > {}; exec sleep 30",
        path.display()
    )));

    // Wait until the process has provably started, then abandon the future —
    // exactly what the bot's tokio::time::timeout does when it fires.
    let pid = tokio::select! {
        out = &mut run => panic!("command finished instead of being abandoned: {out:?}"),
        pid = wait_for_pid(&path) => pid,
    };
    drop(run);

    world.abandoned_pid = Some(pid);
    world.pid_file = Some(pid_file);
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the captured stdout is "(.*)"$"#)]
fn then_stdout(world: &mut SystemWorld, expected: String) {
    let out = world.output.as_ref().expect("no command was run");
    assert_eq!(out.stdout.trim_end(), expected, "stdout mismatch");
}

#[then(regex = r"^the captured exit code is (\d+)$")]
fn then_exit_code(world: &mut SystemWorld, expected: i32) {
    let out = world.output.as_ref().expect("no command was run");
    assert_eq!(out.exit_code, expected, "exit code mismatch");
}

#[then("the spawned process is no longer running")]
async fn then_process_dead(world: &mut SystemWorld) {
    let pid = world.abandoned_pid.expect("no command was abandoned");
    // The kill and the reap are asynchronous — give them a moment.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while process_alive(pid) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "process {pid} is still running after its future was dropped"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    SystemWorld::run("features/system.feature").await;
}
