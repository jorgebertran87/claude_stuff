use changes_detector::detector::{ChangeDetector, CheckResult};
use cucumber::{given, then, when, World};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct DetectorWorld {
    state_dir:   tempfile::TempDir,
    state_file:  std::path::PathBuf,
    detector:    ChangeDetector,
    result_kind: String,   // "bootstrapped" | "no_change" | "changed"
    diff:        String,
}

impl std::fmt::Debug for DetectorWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetectorWorld")
            .field("result_kind", &self.result_kind)
            .finish()
    }
}

impl Default for DetectorWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.state");
        let detector = ChangeDetector::load(&file);
        Self {
            state_dir:   dir,
            state_file:  file,
            detector,
            result_kind: String::new(),
            diff:        String::new(),
        }
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("a fresh change detector")]
fn given_fresh(_world: &mut DetectorWorld) {
    // Default already creates a fresh detector with no state file — nothing to do.
}

#[given(regex = r#"^a change detector seeded with "(.+)"$"#)]
fn given_seeded(world: &mut DetectorWorld, content: String) {
    // Bootstrap the detector so subsequent checks compare against this snapshot.
    world.detector.check(content).unwrap();
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I check "(.+)"$"#)]
fn when_check(world: &mut DetectorWorld, content: String) {
    record_result(world, content);
}

#[when("I reload the detector from the same file")]
fn when_reload(world: &mut DetectorWorld) {
    world.detector = ChangeDetector::load(&world.state_file);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the result is Bootstrapped")]
fn then_bootstrapped(world: &mut DetectorWorld) {
    assert_eq!(world.result_kind, "bootstrapped", "expected Bootstrapped");
}

#[then("the result is NoChange")]
fn then_no_change(world: &mut DetectorWorld) {
    assert_eq!(world.result_kind, "no_change", "expected NoChange");
}

#[then("the result is Changed")]
fn then_changed(world: &mut DetectorWorld) {
    assert_eq!(world.result_kind, "changed", "expected Changed");
}

#[then(regex = r#"^the diff contains "(.+)"$"#)]
fn then_diff_contains(world: &mut DetectorWorld, needle: String) {
    assert!(
        world.diff.contains(&needle),
        "expected diff to contain \"{needle}\", got:\n{}",
        world.diff
    );
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn record_result(world: &mut DetectorWorld, content: String) {
    match world.detector.check(content).unwrap() {
        CheckResult::Bootstrapped    => { world.result_kind = "bootstrapped".into(); }
        CheckResult::NoChange        => { world.result_kind = "no_change".into(); }
        CheckResult::Changed { diff } => {
            world.result_kind = "changed".into();
            world.diff        = diff;
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(
        DetectorWorld::run("features/change_detector.feature"),
    );
}
