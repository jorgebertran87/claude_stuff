use changes_detector::detector::{ChangeDetector, CheckResult};
use cucumber::{given, then, when, World};

const STATE_FILE: &str = "test.state";

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct DetectorWorld {
    state_dir:   tempfile::TempDir,
    detector:    ChangeDetector,
    last_result: Option<CheckResult>,
}

impl std::fmt::Debug for DetectorWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetectorWorld")
            .field("last_result", &self.last_result)
            .finish()
    }
}

impl Default for DetectorWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let detector = ChangeDetector::load(&dir.path().join(STATE_FILE));
        Self { state_dir: dir, detector, last_result: None }
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("a fresh change detector")]
fn given_fresh(_world: &mut DetectorWorld) {}

#[given(regex = r#"^a change detector seeded with "(.+)"$"#)]
fn given_seeded(world: &mut DetectorWorld, content: String) {
    let result = world.detector.check(content).unwrap();
    assert!(
        matches!(result, CheckResult::Bootstrapped),
        "seeding should bootstrap; got: {result:?}"
    );
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I check "(.+)"$"#)]
fn when_check(world: &mut DetectorWorld, content: String) {
    world.last_result = Some(world.detector.check(content).unwrap());
}

#[when("I reload the detector from the same file")]
fn when_reload(world: &mut DetectorWorld) {
    world.detector = ChangeDetector::load(&world.state_dir.path().join(STATE_FILE));
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the result is Bootstrapped")]
fn then_bootstrapped(world: &mut DetectorWorld) {
    assert!(
        matches!(&world.last_result, Some(CheckResult::Bootstrapped)),
        "expected Bootstrapped, got: {:?}", world.last_result
    );
}

#[then("the result is NoChange")]
fn then_no_change(world: &mut DetectorWorld) {
    assert!(
        matches!(&world.last_result, Some(CheckResult::NoChange)),
        "expected NoChange, got: {:?}", world.last_result
    );
}

#[then("the result is Changed")]
fn then_changed(world: &mut DetectorWorld) {
    assert!(
        matches!(&world.last_result, Some(CheckResult::Changed { .. })),
        "expected Changed, got: {:?}", world.last_result
    );
}

#[then(regex = r#"^the diff contains "(.+)"$"#)]
fn then_diff_contains(world: &mut DetectorWorld, needle: String) {
    if let Some(CheckResult::Changed { diff }) = &world.last_result {
        assert!(
            diff.contains(&needle),
            "expected diff to contain \"{needle}\", got:\n{diff}"
        );
    } else {
        panic!("expected Changed result, got: {:?}", world.last_result);
    }
}

#[then(regex = r#"^the diff is exactly "(.+)"$"#)]
fn then_diff_is(world: &mut DetectorWorld, expected: String) {
    // Unescape \n so the feature file can express literal newlines.
    let expected = expected.replace("\\n", "\n");
    if let Some(CheckResult::Changed { diff }) = &world.last_result {
        assert_eq!(diff, &expected, "diff format mismatch");
    } else {
        panic!("expected Changed result, got: {:?}", world.last_result);
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    DetectorWorld::run("features/change_detector.feature").await;
}
