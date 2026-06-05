use std::{
    collections::HashMap,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
};

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use services_controller::{
    control::ServiceController,
    manager::{ServiceManager, ServiceStatus},
    registry::ServiceRegistry,
};
use tokio::sync::Mutex;

// ── Fake controller ─────────────────────────────────────────────────────────

/// In-memory stand-in for any real backend. Tracks running state per service
/// name and can pretend to be unavailable to exercise the error path.
struct FakeController {
    running: Mutex<HashMap<String, bool>>,
    available: AtomicBool,
}

impl FakeController {
    fn new() -> Self {
        Self { running: Mutex::new(HashMap::new()), available: AtomicBool::new(true) }
    }

    fn ensure_available(&self) -> anyhow::Result<()> {
        if self.available.load(Ordering::SeqCst) {
            Ok(())
        } else {
            anyhow::bail!("control backend is unavailable")
        }
    }
}

#[async_trait]
impl ServiceController for FakeController {
    async fn start(&self, service: &str) -> anyhow::Result<()> {
        self.ensure_available()?;
        self.running.lock().await.insert(service.to_string(), true);
        Ok(())
    }

    async fn stop(&self, service: &str) -> anyhow::Result<()> {
        self.ensure_available()?;
        self.running.lock().await.insert(service.to_string(), false);
        Ok(())
    }

    async fn restart(&self, service: &str) -> anyhow::Result<()> {
        self.ensure_available()?;
        self.running.lock().await.insert(service.to_string(), true);
        Ok(())
    }

    async fn status(&self, service: &str) -> anyhow::Result<ServiceStatus> {
        self.ensure_available()?;
        let running = *self.running.lock().await.get(service).unwrap_or(&false);
        Ok(if running { ServiceStatus::Running } else { ServiceStatus::Stopped })
    }
}

// ── World ───────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct ManagerWorld {
    aliases: HashMap<String, String>,
    controller: Arc<FakeController>,
    op_result: Option<Result<(), String>>,
    status_result: Option<ServiceStatus>,
}

impl std::fmt::Debug for ManagerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagerWorld")
            .field("op_result", &self.op_result)
            .field("status_result", &self.status_result)
            .finish()
    }
}

impl Default for ManagerWorld {
    fn default() -> Self {
        Self {
            aliases: HashMap::new(),
            controller: Arc::new(FakeController::new()),
            op_result: None,
            status_result: None,
        }
    }
}

impl ManagerWorld {
    /// Build a fresh manager over the current registry and shared controller.
    fn manager(&self) -> ServiceManager {
        let registry = ServiceRegistry::from_map(self.aliases.clone());
        ServiceManager::new(registry, self.controller.clone())
    }
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a registry mapping alias "([^"]+)" to service "([^"]+)"$"#)]
fn given_mapping(world: &mut ManagerWorld, alias: String, service: String) {
    world.aliases.insert(alias, service);
}

#[given(regex = r#"^the service "([^"]+)" is running$"#)]
async fn given_running(world: &mut ManagerWorld, service: String) {
    world.controller.running.lock().await.insert(service, true);
}

#[given(regex = r#"^the service "([^"]+)" is stopped$"#)]
async fn given_stopped(world: &mut ManagerWorld, service: String) {
    world.controller.running.lock().await.insert(service, false);
}

#[given("the control backend is unavailable")]
fn given_unavailable(world: &mut ManagerWorld) {
    world.controller.available.store(false, Ordering::SeqCst);
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I start "([^"]+)"$"#)]
async fn when_start(world: &mut ManagerWorld, alias: String) {
    world.op_result = Some(world.manager().start(&alias).await.map_err(|e| e.to_string()));
}

#[when(regex = r#"^I stop "([^"]+)"$"#)]
async fn when_stop(world: &mut ManagerWorld, alias: String) {
    world.op_result = Some(world.manager().stop(&alias).await.map_err(|e| e.to_string()));
}

#[when(regex = r#"^I restart "([^"]+)"$"#)]
async fn when_restart(world: &mut ManagerWorld, alias: String) {
    world.op_result = Some(world.manager().restart(&alias).await.map_err(|e| e.to_string()));
}

#[when(regex = r#"^I query the status of "([^"]+)"$"#)]
async fn when_status(world: &mut ManagerWorld, alias: String) {
    match world.manager().status(&alias).await {
        Ok(s) => {
            world.status_result = Some(s);
            world.op_result = Some(Ok(()));
        }
        Err(e) => world.op_result = Some(Err(e.to_string())),
    }
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then("the operation succeeds")]
fn then_ok(world: &mut ManagerWorld) {
    let result = world.op_result.as_ref().expect("no operation was run");
    assert!(result.is_ok(), "expected success, got: {result:?}");
}

#[then("the operation fails")]
fn then_err(world: &mut ManagerWorld) {
    let result = world.op_result.as_ref().expect("no operation was run");
    assert!(result.is_err(), "expected failure, but it succeeded");
}

#[then("the operation fails with an unknown alias error")]
fn then_unknown_alias(world: &mut ManagerWorld) {
    let result = world.op_result.as_ref().expect("no operation was run");
    let err = result.as_ref().expect_err("expected an unknown alias error");
    assert!(err.contains("unknown alias"), "expected an unknown alias error, got: {err}");
}

#[then(regex = r#"^the service "([^"]+)" is running$"#)]
async fn then_service_running(world: &mut ManagerWorld, service: String) {
    let running = *world.controller.running.lock().await.get(&service).unwrap_or(&false);
    assert!(running, "expected service \"{service}\" to be running");
}

#[then(regex = r#"^the service "([^"]+)" is stopped$"#)]
async fn then_service_stopped(world: &mut ManagerWorld, service: String) {
    let running = *world.controller.running.lock().await.get(&service).unwrap_or(&false);
    assert!(!running, "expected service \"{service}\" to be stopped");
}

#[then(regex = r#"^the reported status is "([^"]+)"$"#)]
fn then_status(world: &mut ManagerWorld, expected: String) {
    let status = world.status_result.as_ref().expect("no status was queried");
    assert_eq!(status.as_str(), expected, "status mismatch");
}

// ── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    ManagerWorld::run("features/service_manager.feature").await;
}
