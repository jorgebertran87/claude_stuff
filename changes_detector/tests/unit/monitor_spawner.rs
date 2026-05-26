use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use changes_detector::runner::{MonitorSpawner, Notifier};
use cucumber::{given, then, when, World};
use tokio::sync::Mutex;

// ── Fake notifier ─────────────────────────────────────────────────────────────

struct FakeNotifier;

#[async_trait]
impl Notifier for FakeNotifier {
    async fn notify(&self, _location: &str, _diff: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct SpawnerWorld {
    spawner:     MonitorSpawner,
    bool_result: Option<bool>,
    list_result: Vec<String>,
}

impl std::fmt::Debug for SpawnerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpawnerWorld")
            .field("bool_result", &self.bool_result)
            .field("list_result", &self.list_result)
            .finish()
    }
}

impl Default for SpawnerWorld {
    fn default() -> Self {
        let spawner = MonitorSpawner {
            webdriver_url:    String::new(),
            flaresolverr_url: String::new(),
            notifier:         Arc::new(FakeNotifier),
            data_dir:         PathBuf::new(),
            tasks:            Arc::new(Mutex::new(HashMap::new())),
        };
        Self { spawner, bool_result: None, list_result: Vec::new() }
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("an empty spawner")]
fn given_empty(_world: &mut SpawnerWorld) {}

#[given(regex = r#"^a spawner with a task named "([^"]+)"$"#)]
async fn given_with_task(world: &mut SpawnerWorld, alias: String) {
    // Spawn a never-completing task so we have a valid AbortHandle.
    let handle = tokio::spawn(std::future::pending::<()>());
    world.spawner.tasks.lock().await.insert(alias, handle.abort_handle());
}

#[given(regex = r#"^a spawner with tasks "([^"]+)"$"#)]
async fn given_with_tasks(world: &mut SpawnerWorld, tasks: String) {
    for alias in tasks.split(", ") {
        let handle = tokio::spawn(std::future::pending::<()>());
        world.spawner.tasks.lock().await.insert(alias.to_string(), handle.abort_handle());
    }
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I pause "([^"]+)"$"#)]
async fn when_pause(world: &mut SpawnerWorld, alias: String) {
    world.bool_result = Some(world.spawner.pause(&alias).await);
}

#[when(regex = r#"^I remove "([^"]+)"$"#)]
async fn when_remove(world: &mut SpawnerWorld, alias: String) {
    world.bool_result = Some(world.spawner.remove(&alias).await);
}

#[when("I list aliases")]
async fn when_list(world: &mut SpawnerWorld) {
    world.list_result = world.spawner.list_aliases().await;
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the operation returned true")]
fn then_true(world: &mut SpawnerWorld) {
    assert_eq!(world.bool_result, Some(true), "expected operation to return true");
}

#[then("the operation returned false")]
fn then_false(world: &mut SpawnerWorld) {
    assert_eq!(world.bool_result, Some(false), "expected operation to return false");
}

#[then(regex = r#"^the aliases are "([^"]+)"$"#)]
fn then_aliases(world: &mut SpawnerWorld, expected: String) {
    let expected: Vec<String> = expected.split(", ").map(String::from).collect();
    assert_eq!(world.list_result, expected, "aliases mismatch");
}

#[then("the aliases are empty")]
fn then_aliases_empty(world: &mut SpawnerWorld) {
    assert!(world.list_result.is_empty(), "expected empty aliases, got: {:?}", world.list_result);
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    SpawnerWorld::run("features/monitor_spawner.feature").await;
}
