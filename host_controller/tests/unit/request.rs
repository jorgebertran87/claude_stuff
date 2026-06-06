use cucumber::{given, then, when, World};
use host_controller::request::Request;

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct RequestWorld {
    message: Option<String>,
    result: Option<Request>,
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^the message "(.*)"$"#)]
fn given_message(world: &mut RequestWorld, text: String) {
    world.message = Some(text);
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when("the message is interpreted")]
fn when_interpret(world: &mut RequestWorld) {
    let text = world.message.as_ref().expect("a message must be set");
    world.result = Some(Request::parse(text));
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the result is a command to run "(.*)"$"#)]
fn then_run(world: &mut RequestWorld, expected: String) {
    assert_eq!(world.result, Some(Request::Run(expected)));
}

#[then("the result is a help request")]
fn then_help(world: &mut RequestWorld) {
    assert_eq!(world.result, Some(Request::Help));
}

#[then("the result is ignored")]
fn then_ignored(world: &mut RequestWorld) {
    assert_eq!(world.result, Some(Request::Ignore));
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    RequestWorld::run("features/request.feature").await;
}
