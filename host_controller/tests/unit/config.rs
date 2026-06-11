use std::collections::HashMap;

use cucumber::{given, then, when, World};
use host_controller::config::Config;

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct ConfigWorld {
    values: HashMap<String, String>,
    result: Option<Result<Config, String>>,
}

impl ConfigWorld {
    fn config(&self) -> &Config {
        self.result
            .as_ref()
            .expect("configuration was not loaded")
            .as_ref()
            .expect("configuration failed to load")
    }
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a config value "([^"]+)" of "(.*)"$"#)]
fn given_value(world: &mut ConfigWorld, key: String, value: String) {
    world.values.insert(key, value);
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when("the configuration is loaded")]
fn when_load(world: &mut ConfigWorld) {
    let values = world.values.clone();
    world.result = Some(Config::parse(|k| values.get(k).cloned()).map_err(|e| e.to_string()));
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then("loading succeeds")]
fn then_ok(world: &mut ConfigWorld) {
    let r = world.result.as_ref().expect("configuration was not loaded");
    assert!(r.is_ok(), "expected loading to succeed, got: {r:?}");
}

#[then("loading fails")]
fn then_err(world: &mut ConfigWorld) {
    let r = world.result.as_ref().expect("configuration was not loaded");
    assert!(r.is_err(), "expected loading to fail, but it succeeded");
}

#[then(regex = r#"^the bot token is "(.*)"$"#)]
fn then_token(world: &mut ConfigWorld, expected: String) {
    assert_eq!(world.config().bot_token, expected, "bot token mismatch");
}

#[then(regex = r#"^the allowed chats are "(.*)"$"#)]
fn then_chats(world: &mut ConfigWorld, expected: String) {
    let joined = world
        .config()
        .allowed_chats
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(joined, expected, "allowed chats mismatch");
}

#[then("the allowed chats are empty")]
fn then_chats_empty(world: &mut ConfigWorld) {
    let chats = &world.config().allowed_chats;
    assert!(chats.is_empty(), "expected no allowed chats, got: {chats:?}");
}

#[then(regex = r#"^the ssh target is "(.*)"$"#)]
fn then_target(world: &mut ConfigWorld, expected: String) {
    let c = world.config();
    let target = format!("{}@{}:{}", c.ssh.user, c.ssh.host, c.ssh.port);
    assert_eq!(target, expected, "ssh target mismatch");
}

#[then(regex = r"^the command timeout is (\d+) seconds$")]
fn then_timeout(world: &mut ConfigWorld, secs: u64) {
    assert_eq!(world.config().command_timeout.as_secs(), secs, "timeout mismatch");
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    ConfigWorld::run("features/config.feature").await;
}
