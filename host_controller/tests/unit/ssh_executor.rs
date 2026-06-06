use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use host_controller::executor::{ssh::SshExecutor, CommandExecutor, CommandOutput, CommandRunner};

// ── Fake command runner ──────────────────────────────────────────────────────

/// Records every command it is asked to run and returns canned output, or fails
/// to simulate an ssh transport/spawn failure.
struct FakeRunner {
    calls: Mutex<Vec<Vec<String>>>,
    fail: AtomicBool,
    exit_code: AtomicI32,
    stdout: Mutex<String>,
}

impl FakeRunner {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fail: AtomicBool::new(false),
            exit_code: AtomicI32::new(0),
            stdout: Mutex::new(String::new()),
        }
    }
}

#[async_trait]
impl CommandRunner for FakeRunner {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let mut call = vec![program.to_string()];
        call.extend_from_slice(args);
        self.calls.lock().unwrap().push(call);
        if self.fail.load(Ordering::SeqCst) {
            anyhow::bail!("ssh: connect to host failed");
        }
        Ok(CommandOutput {
            exit_code: self.exit_code.load(Ordering::SeqCst),
            stdout: self.stdout.lock().unwrap().clone(),
            stderr: String::new(),
        })
    }
}

// ── World ────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct SshWorld {
    runner: Arc<FakeRunner>,
    executor: Option<SshExecutor>,
    result: Option<Result<CommandOutput, String>>,
}

impl std::fmt::Debug for SshWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshWorld").field("result", &self.result).finish()
    }
}

impl Default for SshWorld {
    fn default() -> Self {
        Self { runner: Arc::new(FakeRunner::new()), executor: None, result: None }
    }
}

impl SshWorld {
    fn executor(&self) -> &SshExecutor {
        self.executor.as_ref().expect("no ssh executor built")
    }
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^an ssh executor for user "([^"]+)" on host "([^"]+)" port (\d+) with key "([^"]+)" and known hosts "([^"]+)"$"#)]
fn given_executor(
    world: &mut SshWorld,
    user: String,
    host: String,
    port: u16,
    key: String,
    known_hosts: String,
) {
    world.executor = Some(SshExecutor::new(
        world.runner.clone(),
        user,
        host,
        port,
        key,
        known_hosts,
    ));
}

#[given(regex = r#"^the host returns exit code (\d+) with output "([^"]*)"$"#)]
fn given_output(world: &mut SshWorld, code: i32, output: String) {
    world.runner.exit_code.store(code, Ordering::SeqCst);
    *world.runner.stdout.lock().unwrap() = output;
}

#[given("the ssh transport fails")]
fn given_fail(world: &mut SshWorld) {
    world.runner.fail.store(true, Ordering::SeqCst);
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I run "(.+)"$"#)]
async fn when_run(world: &mut SshWorld, command: String) {
    let r = world.executor().execute(&command).await;
    world.result = Some(r.map_err(|e| e.to_string()));
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then(regex = r#"^ssh ran "(.+)"$"#)]
fn then_ssh_ran(world: &mut SshWorld, expected_args: String) {
    let calls = world.runner.calls.lock().unwrap();
    let found = calls
        .iter()
        .any(|call| !call.is_empty() && call[0] == "ssh" && call[1..].join(" ") == expected_args);
    assert!(
        found,
        "expected ssh to run with \"{expected_args}\", got: {:?}",
        *calls
    );
}

#[then("the execution succeeds")]
fn then_ok(world: &mut SshWorld) {
    let r = world.result.as_ref().expect("no execution was made");
    assert!(r.is_ok(), "expected success, got: {r:?}");
}

#[then("the execution fails")]
fn then_err(world: &mut SshWorld) {
    let r = world.result.as_ref().expect("no execution was made");
    assert!(r.is_err(), "expected failure, but it succeeded");
}

#[then(regex = r#"^the returned output is "([^"]*)"$"#)]
fn then_output(world: &mut SshWorld, expected: String) {
    let out = world
        .result
        .as_ref()
        .expect("no execution was made")
        .as_ref()
        .expect("execution failed");
    assert_eq!(out.stdout, expected, "stdout mismatch");
}

#[then(regex = r#"^the returned exit code is (\d+)$"#)]
fn then_exit(world: &mut SshWorld, expected: i32) {
    let out = world
        .result
        .as_ref()
        .expect("no execution was made")
        .as_ref()
        .expect("execution failed");
    assert_eq!(out.exit_code, expected, "exit code mismatch");
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    SshWorld::run("features/ssh_executor.feature").await;
}
