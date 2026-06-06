use cucumber::{given, then, when, World};
use host_controller::executor::CommandOutput;
use host_controller::formatter;

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct FormatterWorld {
    output: Option<CommandOutput>,
    reply: Option<String>,
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^command output with exit code (\d+) and stdout "([^"]*)"$"#)]
fn given_stdout(world: &mut FormatterWorld, code: i32, stdout: String) {
    world.output = Some(CommandOutput { exit_code: code, stdout, stderr: String::new() });
}

#[given(regex = r#"^command output with exit code (\d+) and stderr "([^"]*)"$"#)]
fn given_stderr(world: &mut FormatterWorld, code: i32, stderr: String) {
    world.output = Some(CommandOutput { exit_code: code, stdout: String::new(), stderr });
}

#[given(regex = r"^command output with exit code (\d+) and no output$")]
fn given_no_output(world: &mut FormatterWorld, code: i32) {
    world.output = Some(CommandOutput { exit_code: code, stdout: String::new(), stderr: String::new() });
}

#[given(regex = r"^command output with exit code (\d+) and stdout of (\d+) characters$")]
fn given_long(world: &mut FormatterWorld, code: i32, n: usize) {
    world.output = Some(CommandOutput { exit_code: code, stdout: "x".repeat(n), stderr: String::new() });
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when("the output is formatted")]
fn when_format(world: &mut FormatterWorld) {
    let out = world.output.as_ref().expect("no command output set");
    world.reply = Some(formatter::format(out));
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the reply is "([^"]*)"$"#)]
fn then_reply_is(world: &mut FormatterWorld, expected: String) {
    assert_eq!(world.reply.as_ref().expect("no reply"), &expected);
}

#[then(regex = r#"^the reply contains "([^"]*)"$"#)]
fn then_reply_contains(world: &mut FormatterWorld, needle: String) {
    let reply = world.reply.as_ref().expect("no reply");
    assert!(reply.contains(&needle), "reply did not contain {needle:?}: {reply:?}");
}

#[then(regex = r"^the reply is at most (\d+) characters$")]
fn then_reply_atmost(world: &mut FormatterWorld, max: usize) {
    let len = world.reply.as_ref().expect("no reply").chars().count();
    assert!(len <= max, "reply length {len} exceeds {max}");
}

#[then(regex = r#"^the reply ends with "([^"]*)"$"#)]
fn then_reply_ends(world: &mut FormatterWorld, suffix: String) {
    let reply = world.reply.as_ref().expect("no reply");
    assert!(reply.ends_with(&suffix), "reply did not end with {suffix:?}");
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    FormatterWorld::run("features/formatter.feature").await;
}
