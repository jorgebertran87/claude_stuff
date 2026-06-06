use cucumber::{given, then, when, World};
use host_controller::authorizer::Authorizer;

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct AuthWorld {
    authorizer: Option<Authorizer>,
    result: Option<bool>,
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given(regex = r"^an allowlist of chats (.+)$")]
fn given_allowlist(world: &mut AuthWorld, list: String) {
    let ids = list
        .split(',')
        .map(|s| s.trim().parse::<i64>().expect("chat id must be an integer"))
        .collect::<Vec<_>>();
    world.authorizer = Some(Authorizer::new(ids));
}

#[given("an empty allowlist")]
fn given_empty(world: &mut AuthWorld) {
    world.authorizer = Some(Authorizer::new(Vec::<i64>::new()));
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when(regex = r"^chat (-?\d+) is checked$")]
fn when_checked(world: &mut AuthWorld, chat_id: i64) {
    let authorized = world
        .authorizer
        .as_ref()
        .expect("an allowlist must be configured")
        .is_authorized(chat_id);
    world.result = Some(authorized);
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then("the chat is authorized")]
fn then_authorized(world: &mut AuthWorld) {
    assert_eq!(world.result, Some(true), "expected the chat to be authorized");
}

#[then("the chat is denied")]
fn then_denied(world: &mut AuthWorld) {
    assert_eq!(world.result, Some(false), "expected the chat to be denied");
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    AuthWorld::run("features/authorization.feature").await;
}
