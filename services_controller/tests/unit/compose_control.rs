use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use services_controller::{
    control::{compose::ComposeController, CommandOutput, CommandRunner, ServiceController},
    manager::ServiceStatus,
};

// ── Fake command runner ──────────────────────────────────────────────────────

/// Records every command it is asked to run and returns canned output.
struct FakeRunner {
    calls: Mutex<Vec<Vec<String>>>,
    success: AtomicBool,
    stdout: Mutex<String>,
}

impl FakeRunner {
    fn new() -> Self {
        Self { calls: Mutex::new(Vec::new()), success: AtomicBool::new(true), stdout: Mutex::new(String::new()) }
    }
}

#[async_trait]
impl CommandRunner for FakeRunner {
    async fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let mut call = vec![program.to_string()];
        call.extend_from_slice(args);
        self.calls.lock().unwrap().push(call);
        Ok(CommandOutput {
            success: self.success.load(Ordering::SeqCst),
            stdout: self.stdout.lock().unwrap().clone(),
            stderr: String::new(),
        })
    }
}

// ── World ────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct ComposeWorld {
    runner: Arc<FakeRunner>,
    controller: Option<ComposeController>,
    op_result: Option<Result<(), String>>,
    status_result: Option<ServiceStatus>,
}

impl std::fmt::Debug for ComposeWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComposeWorld")
            .field("op_result", &self.op_result)
            .field("status_result", &self.status_result)
            .finish()
    }
}

impl Default for ComposeWorld {
    fn default() -> Self {
        Self { runner: Arc::new(FakeRunner::new()), controller: None, op_result: None, status_result: None }
    }
}

impl ComposeWorld {
    fn controller(&self) -> &ComposeController {
        self.controller.as_ref().expect("no compose controller built")
    }

    fn record_op(&mut self, result: anyhow::Result<()>) {
        self.op_result = Some(result.map_err(|e| e.to_string()));
    }
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given("a compose controller")]
fn given_controller(world: &mut ComposeWorld) {
    world.controller = Some(ComposeController::new(world.runner.clone()));
}

#[given("docker compose ps reports a running container")]
fn given_ps_running(world: &mut ComposeWorld) {
    *world.runner.stdout.lock().unwrap() = r#"{"Service":"web","State":"running"}"#.to_string();
}

#[given("docker compose ps reports an exited container")]
fn given_ps_exited(world: &mut ComposeWorld) {
    *world.runner.stdout.lock().unwrap() = r#"{"Service":"web","State":"exited"}"#.to_string();
}

#[given("docker compose commands fail")]
fn given_fail(world: &mut ComposeWorld) {
    world.runner.success.store(false, Ordering::SeqCst);
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I start the service in "([^"]+)"$"#)]
async fn when_start(world: &mut ComposeWorld, dir: String) {
    let r = world.controller().start(&dir).await;
    world.record_op(r);
}

#[when(regex = r#"^I stop the service in "([^"]+)"$"#)]
async fn when_stop(world: &mut ComposeWorld, dir: String) {
    let r = world.controller().stop(&dir).await;
    world.record_op(r);
}

#[when(regex = r#"^I restart the service in "([^"]+)"$"#)]
async fn when_restart(world: &mut ComposeWorld, dir: String) {
    let r = world.controller().restart(&dir).await;
    world.record_op(r);
}

#[when(regex = r#"^I query the status of the service in "([^"]+)"$"#)]
async fn when_status(world: &mut ComposeWorld, dir: String) {
    match world.controller().status(&dir).await {
        Ok(s) => {
            world.status_result = Some(s);
            world.op_result = Some(Ok(()));
        }
        Err(e) => world.op_result = Some(Err(e.to_string())),
    }
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then(regex = r#"^docker ran "(.+)"$"#)]
fn then_docker_ran(world: &mut ComposeWorld, expected_args: String) {
    let calls = world.runner.calls.lock().unwrap();
    let found = calls.iter().any(|call| {
        !call.is_empty() && call[0] == "docker" && call[1..].join(" ") == expected_args
    });
    assert!(
        found,
        "expected docker to be run with \"{expected_args}\", got: {:?}",
        *calls
    );
}

#[then("the control call succeeds")]
fn then_ok(world: &mut ComposeWorld) {
    let result = world.op_result.as_ref().expect("no control call was made");
    assert!(result.is_ok(), "expected success, got: {result:?}");
}

#[then("the control call fails")]
fn then_err(world: &mut ComposeWorld) {
    let result = world.op_result.as_ref().expect("no control call was made");
    assert!(result.is_err(), "expected failure, but it succeeded");
}

#[then(regex = r#"^the reported status is "([^"]+)"$"#)]
fn then_status(world: &mut ComposeWorld, expected: String) {
    let status = world.status_result.as_ref().expect("no status was queried");
    assert_eq!(status.as_str(), expected, "status mismatch");
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    ComposeWorld::run("features/compose_control.feature").await;
}
