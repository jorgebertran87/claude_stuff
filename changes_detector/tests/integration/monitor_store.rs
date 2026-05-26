use changes_detector::monitor::{MonitorConfig, MonitorMode, MonitorStore, SourceType};
use cucumber::{given, then, when, World};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct StoreWorld {
    data_dir: tempfile::TempDir,
    store:    MonitorStore,
}

impl std::fmt::Debug for StoreWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreWorld")
            .field("monitors", &self.store.all().len())
            .finish()
    }
}

impl Default for StoreWorld {
    fn default() -> Self {
        let dir   = tempfile::tempdir().unwrap();
        let store = MonitorStore::load(dir.path());
        Self { data_dir: dir, store }
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("a monitor store in a temporary directory")]
fn given_store(_world: &mut StoreWorld) {}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I add monitor "([^"]+)" watching "([^"]+)" selector "([^"]+)" every (\d+) seconds$"#)]
fn when_add(world: &mut StoreWorld, alias: String, url: String, selector: String, interval: u64) {
    world.store.add(MonitorConfig {
        alias,
        url:           Some(url),
        selector,
        interval_secs: interval,
        mode:          MonitorMode::Content,
        source_type:   SourceType::Browser,
        paused:        false,
    }).unwrap();
}

#[when(regex = r#"^I remove monitor "([^"]+)"$"#)]
fn when_remove(world: &mut StoreWorld, alias: String) {
    world.store.remove(&alias).unwrap();
}

#[when(regex = r#"^I pause monitor "([^"]+)"$"#)]
fn when_pause(world: &mut StoreWorld, alias: String) {
    world.store.set_paused(&alias, true).unwrap();
}

#[when(regex = r#"^I resume monitor "([^"]+)"$"#)]
fn when_resume(world: &mut StoreWorld, alias: String) {
    world.store.set_paused(&alias, false).unwrap();
}

#[when("I reload the store from disk")]
fn when_reload(world: &mut StoreWorld) {
    world.store = MonitorStore::load(world.data_dir.path());
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r"^the store has (\d+) monitors$")]
fn then_count(world: &mut StoreWorld, expected: usize) {
    assert_eq!(world.store.all().len(), expected, "monitor count mismatch");
}

#[then(regex = r#"^the store contains monitor "([^"]+)"$"#)]
fn then_contains(world: &mut StoreWorld, alias: String) {
    let found = world.store.all().iter().any(|m| m.alias == alias);
    assert!(found, "expected to find monitor \"{alias}\" in store");
}

#[then(regex = r#"^monitor "([^"]+)" is paused$"#)]
fn then_paused(world: &mut StoreWorld, alias: String) {
    assert!(find_monitor(&world.store, &alias).paused, "expected monitor \"{alias}\" to be paused");
}

#[then(regex = r#"^monitor "([^"]+)" is not paused$"#)]
fn then_not_paused(world: &mut StoreWorld, alias: String) {
    assert!(!find_monitor(&world.store, &alias).paused, "expected monitor \"{alias}\" to not be paused");
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn find_monitor<'a>(store: &'a MonitorStore, alias: &str) -> &'a MonitorConfig {
    store.all().iter().find(|m| m.alias == alias)
        .unwrap_or_else(|| panic!("monitor \"{alias}\" not found"))
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    StoreWorld::run("features/monitor_store.feature").await;
}
