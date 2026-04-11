use cucumber::{given, when, then, World};

#[derive(Debug, Default, World)]
pub struct CliWorld {
    args: Vec<String>,
    result: Option<Result<voice_assistant::cli::CliArgs, String>>,
}

// ── Steps ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^the CLI arguments are "(.+)"$"#)]
fn given_cli_args(world: &mut CliWorld, raw: String) {
    // Parse shell-style arguments: --order "pon música" becomes ["--order", "pon música"]
    // Simple approach: split on whitespace but rejoin tokens after a flag like --order
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    let mut args = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].starts_with("--") {
            args.push(tokens[i].to_string());
            if tokens[i] == "--order" && i + 1 < tokens.len() {
                // Collect all following tokens until the next flag as the order value
                i += 1;
                let mut value_parts = Vec::new();
                while i < tokens.len() && !tokens[i].starts_with("--") {
                    value_parts.push(tokens[i]);
                    i += 1;
                }
                if !value_parts.is_empty() {
                    args.push(value_parts.join(" "));
                }
                continue;
            }
        } else {
            args.push(tokens[i].to_string());
        }
        i += 1;
    }
    world.args = args;
}

#[given("no CLI arguments are provided")]
fn given_no_args(world: &mut CliWorld) {
    world.args.clear();
}

#[when("the arguments are parsed")]
fn when_parsed(world: &mut CliWorld) {
    world.result = Some(voice_assistant::cli::parse_args(&world.args));
}

#[then(regex = r#"^the mode is DirectOrder with text "(.+)"$"#)]
fn then_direct_order(world: &mut CliWorld, expected: String) {
    match world.result.as_ref().unwrap() {
        Ok(voice_assistant::cli::CliArgs::DirectOrder(text)) => {
            assert_eq!(text, &expected);
        }
        other => panic!("expected DirectOrder(\"{expected}\"), got {other:?}"),
    }
}

#[then("parsing fails with an error")]
fn then_parsing_fails(world: &mut CliWorld) {
    assert!(
        world.result.as_ref().unwrap().is_err(),
        "expected Err, got Ok"
    );
}

#[then("the mode is TelegramMode")]
fn then_telegram_mode(world: &mut CliWorld) {
    assert!(
        matches!(world.result.as_ref().unwrap(), Ok(voice_assistant::cli::CliArgs::TelegramMode)),
        "expected TelegramMode"
    );
}

#[then("the mode is ListenMode")]
fn then_listen_mode(world: &mut CliWorld) {
    assert!(
        matches!(world.result.as_ref().unwrap(), Ok(voice_assistant::cli::CliArgs::ListenMode)),
        "expected ListenMode"
    );
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(
        CliWorld::run("features/cli_direct_order.feature"),
    );
}
