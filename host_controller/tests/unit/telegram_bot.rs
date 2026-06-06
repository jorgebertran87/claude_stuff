use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use host_controller::{
    authorizer::Authorizer,
    executor::{CommandExecutor, CommandOutput},
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

// ── Fake executor ────────────────────────────────────────────────────────────

/// Records the commands it is asked to run and returns canned output. Can be
/// told to hang (to trigger a timeout) or to fail (to simulate an ssh error).
#[derive(Default)]
struct FakeExecutor {
    commands: Mutex<Vec<String>>,
    stdout: Mutex<String>,
    exit_code: AtomicI32,
    hang: AtomicBool,
    fail: AtomicBool,
}

#[async_trait]
impl CommandExecutor for FakeExecutor {
    async fn execute(&self, command: &str) -> anyhow::Result<CommandOutput> {
        self.commands.lock().unwrap().push(command.to_string());
        if self.hang.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
        if self.fail.load(Ordering::SeqCst) {
            anyhow::bail!("ssh: connect to host failed");
        }
        Ok(CommandOutput {
            exit_code: self.exit_code.load(Ordering::SeqCst),
            stdout: self.stdout.lock().unwrap().clone(),
            stderr: String::new(),
        })
    }
}

// ── World ────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct BotWorld {
    gateway: Arc<FakeGateway>,
    executor: Arc<FakeExecutor>,
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
            executor: Arc::new(FakeExecutor::default()),
            bot: None,
            offset: 0,
        }
    }
}

impl BotWorld {
    fn build_bot(&mut self, allowed: Vec<i64>, timeout: Duration) {
        self.bot = Some(TelegramBot::new(
            self.gateway.clone(),
            Authorizer::new(allowed),
            self.executor.clone(),
            timeout,
        ));
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

#[given(regex = r"^a bot that allows chat (\d+)$")]
fn given_bot(world: &mut BotWorld, chat: i64) {
    world.build_bot(vec![chat], Duration::from_secs(30));
}

#[given("a bot whose commands time out quickly")]
fn given_bot_quick_timeout(world: &mut BotWorld) {
    world.build_bot(vec![1], Duration::from_millis(20));
}

#[given(regex = r#"^the host returns the output "(.*)"$"#)]
fn given_output(world: &mut BotWorld, output: String) {
    *world.executor.stdout.lock().unwrap() = output;
}

#[given("the host does not respond in time")]
fn given_hang(world: &mut BotWorld) {
    world.executor.hang.store(true, Ordering::SeqCst);
}

#[given("the host is unreachable")]
fn given_fail(world: &mut BotWorld) {
    world.executor.fail.store(true, Ordering::SeqCst);
}

#[given(regex = r#"^a command "(.*)" from chat (\d+)$"#)]
fn given_command(world: &mut BotWorld, text: String, chat_id: i64) {
    let id = world.offset + 1;
    world.push_update(id, chat_id, &text);
}

#[given(regex = r#"^a command with id (\d+) "(.*)" from chat (\d+)$"#)]
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

#[then(regex = r#"^a reply to chat (\d+) contains "(.*)"$"#)]
fn then_reply_contains(world: &mut BotWorld, chat_id: i64, needle: String) {
    let posted = world.gateway.posted.lock().unwrap();
    let found = posted.iter().any(|(id, text)| *id == chat_id && text.contains(&needle));
    assert!(
        found,
        "expected a reply to chat {chat_id} containing {needle:?}, got: {:?}",
        *posted
    );
}

#[then("no reply is posted")]
fn then_no_reply(world: &mut BotWorld) {
    let posted = world.gateway.posted.lock().unwrap();
    assert!(posted.is_empty(), "expected no replies, got: {:?}", *posted);
}

#[then(regex = r#"^the host ran "(.*)"$"#)]
fn then_host_ran(world: &mut BotWorld, expected: String) {
    let commands = world.executor.commands.lock().unwrap();
    assert!(
        commands.iter().any(|c| c == &expected),
        "expected the host to run {expected:?}, got: {:?}",
        *commands
    );
}

#[then("no command is run on the host")]
fn then_nothing_ran(world: &mut BotWorld) {
    let commands = world.executor.commands.lock().unwrap();
    assert!(commands.is_empty(), "expected no commands, got: {:?}", *commands);
}

#[then(regex = r"^the offset is (\d+)$")]
fn then_offset(world: &mut BotWorld, expected: i64) {
    assert_eq!(world.offset, expected, "offset mismatch");
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    BotWorld::run("features/telegram_bot.feature").await;
}
