use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use services_controller::{
    control::ServiceController,
    manager::{ServiceManager, ServiceStatus},
    registry::ServiceRegistry,
    telegram::{TelegramBot, TelegramGateway, TelegramUpdate},
};

// ── Fake gateway ─────────────────────────────────────────────────────────────

#[derive(Default)]
struct FakeGateway {
    updates: Mutex<Vec<TelegramUpdate>>,
    posted: Mutex<Vec<(i64, String)>>,
}

#[async_trait]
impl TelegramGateway for FakeGateway {
    async fn fetch_updates(&self, _offset: i64) -> Vec<TelegramUpdate> {
        self.updates.lock().unwrap().drain(..).collect()
    }

    async fn post_message(&self, chat_id: i64, text: &str) {
        self.posted.lock().unwrap().push((chat_id, text.to_string()));
    }
}

// ── Fake controller ──────────────────────────────────────────────────────────

#[derive(Default)]
struct FakeController {
    running: Mutex<HashMap<String, bool>>,
}

#[async_trait]
impl ServiceController for FakeController {
    async fn start(&self, service: &str) -> anyhow::Result<()> {
        self.running.lock().unwrap().insert(service.to_string(), true);
        Ok(())
    }

    async fn stop(&self, service: &str) -> anyhow::Result<()> {
        self.running.lock().unwrap().insert(service.to_string(), false);
        Ok(())
    }

    async fn restart(&self, service: &str) -> anyhow::Result<()> {
        self.running.lock().unwrap().insert(service.to_string(), true);
        Ok(())
    }

    async fn status(&self, service: &str) -> anyhow::Result<ServiceStatus> {
        let running = *self.running.lock().unwrap().get(service).unwrap_or(&false);
        Ok(if running { ServiceStatus::Running } else { ServiceStatus::Stopped })
    }
}

// ── World ────────────────────────────────────────────────────────────────────

// Every scenario maps alias "web" to this underlying service name.
const SERVICE: &str = "nginx";

#[derive(World)]
pub struct BotWorld {
    gateway: Arc<FakeGateway>,
    controller: Arc<FakeController>,
    bot: Option<TelegramBot>,
    offset: i64,
}

impl std::fmt::Debug for BotWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotWorld").field("offset", &self.offset).finish()
    }
}

impl Default for BotWorld {
    fn default() -> Self {
        Self {
            gateway: Arc::new(FakeGateway::default()),
            controller: Arc::new(FakeController::default()),
            bot: None,
            offset: 0,
        }
    }
}

impl BotWorld {
    fn build_bot(&mut self, alias: &str, running: bool, allowed: Vec<i64>) {
        self.controller.running.lock().unwrap().insert(SERVICE.to_string(), running);
        let mut map = HashMap::new();
        map.insert(alias.to_string(), SERVICE.to_string());
        let manager = Arc::new(ServiceManager::new(
            ServiceRegistry::from_map(map),
            self.controller.clone(),
        ));
        self.bot = Some(TelegramBot::new(self.gateway.clone(), manager, allowed));
    }

    fn push_update(&mut self, update_id: i64, chat_id: i64, text: &str) {
        self.gateway.updates.lock().unwrap().push(TelegramUpdate {
            update_id,
            chat_id,
            text: text.to_string(),
        });
    }
}

// ── Given ────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a service bot mapping alias "([^"]+)" to a (running|stopped) service$"#)]
fn given_bot(world: &mut BotWorld, alias: String, state: String) {
    world.build_bot(&alias, state == "running", vec![]);
}

#[given(regex = r#"^a service bot mapping alias "([^"]+)" to a (running|stopped) service that only allows chat (\d+)$"#)]
fn given_bot_restricted(world: &mut BotWorld, alias: String, state: String, chat: i64) {
    world.build_bot(&alias, state == "running", vec![chat]);
}

#[given(regex = r#"^a command "(.+)" from chat (\d+)$"#)]
fn given_command(world: &mut BotWorld, text: String, chat_id: i64) {
    let id = world.offset + 1;
    world.push_update(id, chat_id, &text);
}

#[given(regex = r#"^a command with id (\d+) "(.+)" from chat (\d+)$"#)]
fn given_command_with_id(world: &mut BotWorld, id: i64, text: String, chat_id: i64) {
    world.push_update(id, chat_id, &text);
}

// ── When ─────────────────────────────────────────────────────────────────────

#[when("the bot processes the updates")]
async fn when_process(world: &mut BotWorld) {
    let mut offset = world.offset;
    world.bot.as_ref().unwrap().run_once(&mut offset).await;
    world.offset = offset;
}

// ── Then ─────────────────────────────────────────────────────────────────────

#[then(regex = r#"^a reply to chat (\d+) contains "(.+)"$"#)]
fn then_reply_contains(world: &mut BotWorld, chat_id: i64, needle: String) {
    let posted = world.gateway.posted.lock().unwrap();
    let found = posted.iter().any(|(id, text)| *id == chat_id && text.contains(&needle));
    assert!(
        found,
        "expected a reply to chat {chat_id} containing \"{needle}\", got: {:?}",
        *posted
    );
}

#[then("no reply is posted")]
fn then_no_reply(world: &mut BotWorld) {
    let posted = world.gateway.posted.lock().unwrap();
    assert!(posted.is_empty(), "expected no replies, got: {:?}", *posted);
}

#[then(regex = r#"^the service behind "([^"]+)" is running$"#)]
fn then_running(world: &mut BotWorld, _alias: String) {
    let running = *world.controller.running.lock().unwrap().get(SERVICE).unwrap_or(&false);
    assert!(running, "expected the service to be running");
}

#[then(regex = r#"^the service behind "([^"]+)" is stopped$"#)]
fn then_stopped(world: &mut BotWorld, _alias: String) {
    let running = *world.controller.running.lock().unwrap().get(SERVICE).unwrap_or(&false);
    assert!(!running, "expected the service to be stopped");
}

#[then(regex = r"^the offset is (\d+)$")]
fn then_offset(world: &mut BotWorld, expected: i64) {
    assert_eq!(world.offset, expected, "offset mismatch");
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    BotWorld::run("features/telegram_command.feature").await;
}
