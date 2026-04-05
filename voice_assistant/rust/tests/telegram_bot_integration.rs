use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cucumber::{given, when, then, World};

use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::telegram_bot::{TelegramBot, TelegramGateway, TelegramUpdate};

// ── Fake gateway ──────────��────────────────────────────────────────────────────

#[derive(Default)]
struct FakeGateway {
    updates: Mutex<Vec<TelegramUpdate>>,
    posted: Mutex<Vec<(i64, String)>>,
    voices: Mutex<Vec<(i64, Vec<u8>)>>,
}

impl TelegramGateway for FakeGateway {
    fn fetch_updates(&self, _offset: i64) -> Vec<TelegramUpdate> {
        self.updates.lock().unwrap().drain(..).collect()
    }

    fn post_message(&self, chat_id: i64, text: &str) {
        self.posted.lock().unwrap().push((chat_id, text.to_string()));
    }

    fn send_voice(&self, chat_id: i64, data: &[u8]) {
        self.voices.lock().unwrap().push((chat_id, data.to_vec()));
    }
}

// ── Fake handler ───────────────────────────────────────���───────────────────────

struct FakeHandler {
    received: Mutex<Vec<String>>,
}

impl FakeHandler {
    fn new() -> Self {
        Self { received: Mutex::new(vec![]) }
    }
}

impl OrderHandler for FakeHandler {
    fn handle(&self, order: &str) -> String {
        self.received.lock().unwrap().push(order.to_string());
        format!("reply to: {order}")
    }

    fn reset_session(&self) {}
}

// ── World ──────────────────────────────────���───────────────────────────────────

#[derive(World)]
pub struct TelegramWorld {
    gateway: Arc<FakeGateway>,
    bot: Option<TelegramBot>,
    handler: Arc<FakeHandler>,
    handlers: HashMap<i64, Arc<dyn OrderHandler>>,
    voice_mode_chats: HashSet<i64>,
    pending_auth_chats: HashMap<i64, Instant>,
    offset: i64,
}

impl std::fmt::Debug for TelegramWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramWorld")
            .field("offset", &self.offset)
            .finish()
    }
}

impl Default for TelegramWorld {
    fn default() -> Self {
        let gateway = Arc::new(FakeGateway::default());
        let handler = Arc::new(FakeHandler::new());
        Self {
            gateway: Arc::clone(&gateway),
            bot: None,
            handler: Arc::clone(&handler),
            handlers: HashMap::new(),
            voice_mode_chats: HashSet::new(),
            pending_auth_chats: HashMap::new(),
            offset: 0,
        }
    }
}

// ── Given steps ��───────────────────────────��───────────────────────────────────

#[given("a TelegramBot with a fake gateway")]
fn given_bot(world: &mut TelegramWorld) {
    let gateway = Arc::new(FakeGateway::default());
    world.gateway = Arc::clone(&gateway);
    world.bot = Some(TelegramBot::with_injectable(
        Box::new(FakeGatewayWrapper(Arc::clone(&gateway))),
        vec![],
    ));
}

#[given(regex = r"^a TelegramBot with a fake gateway allowing only chat (\d+)$")]
fn given_bot_restricted(world: &mut TelegramWorld, chat_id: i64) {
    let gateway = Arc::new(FakeGateway::default());
    world.gateway = Arc::clone(&gateway);
    world.bot = Some(TelegramBot::with_injectable(
        Box::new(FakeGatewayWrapper(Arc::clone(&gateway))),
        vec![chat_id],
    ));
}

#[given(regex = r#"^an update with text "(.+)" from chat (\d+)$"#)]
fn given_update(world: &mut TelegramWorld, text: String, chat_id: i64) {
    world.gateway.updates.lock().unwrap().push(TelegramUpdate {
        update_id: world.offset + 1,
        chat_id,
        text,
    });
}

#[given(regex = r"^an update with id (\d+) and text \"(.+)\" from chat (\d+)$")]
fn given_update_with_id(world: &mut TelegramWorld, id: i64, text: String, chat_id: i64) {
    world.gateway.updates.lock().unwrap().push(TelegramUpdate {
        update_id: id,
        chat_id,
        text,
    });
}

#[given(regex = r"^a handler exists for chat (\d+)$")]
fn given_handler_exists(world: &mut TelegramWorld, chat_id: i64) {
    let handler: Arc<dyn OrderHandler> = Arc::clone(&world.handler) as Arc<dyn OrderHandler>;
    world.handlers.insert(chat_id, handler);
}

// ── When steps ──────────────────────��──────────────────────���───────────────────

#[when("run_once processes the updates")]
fn when_run_once(world: &mut TelegramWorld) {
    let handler = Arc::clone(&world.handler);
    let make_handler: &dyn Fn() -> Arc<dyn OrderHandler> = &|| Arc::clone(&handler) as Arc<dyn OrderHandler>;
    let speak_text: &dyn Fn(&str) = &|_| {};
    let on_voice: &dyn Fn() = &|| {};

    world.bot.as_ref().unwrap().run_once(
        make_handler,
        &mut world.handlers,
        &mut world.voice_mode_chats,
        &mut world.pending_auth_chats,
        &mut world.offset,
        speak_text,
        on_voice,
    );
}

#[when(regex = r#"^run_once processes another "(.+)" from chat (\d+)$"#)]
fn when_run_once_again(world: &mut TelegramWorld, text: String, chat_id: i64) {
    world.gateway.updates.lock().unwrap().push(TelegramUpdate {
        update_id: world.offset + 1,
        chat_id,
        text,
    });

    let handler = Arc::clone(&world.handler);
    let make_handler: &dyn Fn() -> Arc<dyn OrderHandler> = &|| Arc::clone(&handler) as Arc<dyn OrderHandler>;
    let speak_text: &dyn Fn(&str) = &|_| {};
    let on_voice: &dyn Fn() = &|| {};

    world.bot.as_ref().unwrap().run_once(
        make_handler,
        &mut world.handlers,
        &mut world.voice_mode_chats,
        &mut world.pending_auth_chats,
        &mut world.offset,
        speak_text,
        on_voice,
    );
}

// ── Then steps ────────────────���─────────────────────────────────��──────────────

#[then(regex = r#"^the gateway posted a message to chat (\d+) containing "(.+)"$"#)]
fn then_posted_containing(world: &mut TelegramWorld, chat_id: i64, needle: String) {
    let posted = world.gateway.posted.lock().unwrap();
    let found = posted.iter().any(|(id, text)| *id == chat_id && text.contains(&needle));
    assert!(
        found,
        "expected message to chat {chat_id} containing \"{needle}\", got: {:?}",
        *posted
    );
}

#[then(regex = r"^the gateway posted a message to chat (\d+)$")]
fn then_posted_to_chat(world: &mut TelegramWorld, chat_id: i64) {
    let posted = world.gateway.posted.lock().unwrap();
    let found = posted.iter().any(|(id, _)| *id == chat_id);
    assert!(found, "expected message to chat {chat_id}, got: {:?}", *posted);
}

#[then("the gateway posted no messages")]
fn then_no_messages(world: &mut TelegramWorld) {
    let posted = world.gateway.posted.lock().unwrap();
    assert!(posted.is_empty(), "expected no messages, got: {:?}", *posted);
}

#[then(regex = r#"^the handler received "(.+)"$"#)]
fn then_handler_received(world: &mut TelegramWorld, expected: String) {
    let received = world.handler.received.lock().unwrap();
    assert!(
        received.iter().any(|r| r.contains(&expected)),
        "handler should have received \"{expected}\", got: {:?}",
        *received
    );
}

#[then(regex = r"^the offset is (\d+)$")]
fn then_offset(world: &mut TelegramWorld, expected: i64) {
    assert_eq!(world.offset, expected, "offset mismatch");
}

// ── Gateway wrapper to share Arc ───���───────────────────────────────────────────

struct FakeGatewayWrapper(Arc<FakeGateway>);

impl TelegramGateway for FakeGatewayWrapper {
    fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate> {
        self.0.fetch_updates(offset)
    }
    fn post_message(&self, chat_id: i64, text: &str) {
        self.0.post_message(chat_id, text);
    }
    fn send_voice(&self, chat_id: i64, data: &[u8]) {
        self.0.send_voice(chat_id, data);
    }
}

fn main() {
    futures::executor::block_on(
        TelegramWorld::run("features/telegram_bot_integration.feature"),
    );
}
