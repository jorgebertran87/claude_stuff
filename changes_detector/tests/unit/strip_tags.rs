use changes_detector::runner::strip_tags;
use cucumber::{then, when, World};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct StripWorld {
    result: String,
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I strip tags from "(.+)"$"#)]
fn when_strip(world: &mut StripWorld, html: String) {
    world.result = strip_tags(&html);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the stripped result is "(.+)"$"#)]
fn then_result(world: &mut StripWorld, expected: String) {
    // Unescape \" so the feature file can test a literal double-quote in output.
    let expected = expected.replace("\\\"", "\"");
    assert_eq!(world.result, expected, "strip_tags result mismatch");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    StripWorld::run("features/strip_tags.feature").await;
}
