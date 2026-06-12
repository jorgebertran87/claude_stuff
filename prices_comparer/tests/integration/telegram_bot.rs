use std::collections::HashMap;

use async_trait::async_trait;
use cucumber::{given, then, when, World};
use prices_comparer::comparer::StoreSource;
use prices_comparer::telegram::TelegramBot;
use serde_json::json;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BOT_TOKEN: &str = "TESTTOKEN";
const CONFIGURED_CHAT: i64 = 42;
const UPDATE_ID: i64 = 7;

// ── Fake store ────────────────────────────────────────────────────────────────

struct FakeStore {
    name: String,
    prices: HashMap<String, u64>,
}

#[async_trait]
impl StoreSource for FakeStore {
    fn name(&self) -> &str {
        &self.name
    }

    async fn price_cents(&self, product: &str) -> anyhow::Result<Option<u64>> {
        Ok(self.prices.get(product).copied())
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World, Default)]
pub struct BotWorld {
    server: Option<MockServer>,
    stores: Vec<Box<dyn StoreSource>>,
    bot: Option<TelegramBot>,
}

impl std::fmt::Debug for BotWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotWorld").finish()
    }
}

/// Parse a price like "1.10" into cents without going through floats.
fn cents(price: &str) -> u64 {
    let (euros, cents) = price.split_once('.').unwrap_or((price, "0"));
    let euros: u64 = euros.parse().expect("euros");
    let cents: u64 = format!("{cents:0<2}").parse().expect("cents");
    euros * 100 + cents
}

async fn mount_telegram(world: &mut BotWorld, text: &str, chat_id: i64) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/bot.+/getUpdates$"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "ok": true,
                "result": [{
                    "update_id": UPDATE_ID,
                    "message": { "chat": { "id": chat_id }, "text": text }
                }]
            })),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path_regex(r"^/bot.+/sendMessage$"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "ok": true, "result": { "message_id": 1 } })),
        )
        .mount(&server)
        .await;
    world.server = Some(server);
}

/// All requests the mock Telegram API received on the given endpoint.
async fn requests_to(world: &BotWorld, endpoint: &str) -> Vec<wiremock::Request> {
    world
        .server
        .as_ref()
        .expect("mock server not started")
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.url.path().ends_with(endpoint))
        .collect()
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock Telegram API delivering "([^"]+)" from the configured chat$"#)]
async fn given_message_configured(world: &mut BotWorld, text: String) {
    mount_telegram(world, &text, CONFIGURED_CHAT).await;
}

#[given(regex = r#"^a mock Telegram API delivering "([^"]+)" from an unknown chat$"#)]
async fn given_message_unknown(world: &mut BotWorld, text: String) {
    mount_telegram(world, &text, 99).await;
}

#[given(regex = r#"^a store "([^"]+)" selling "([^"]+)" at (\d+\.\d+) and "([^"]+)" at (\d+\.\d+)$"#)]
fn given_store_two_products(
    world: &mut BotWorld,
    store: String,
    product_a: String,
    price_a: String,
    product_b: String,
    price_b: String,
) {
    let prices = HashMap::from([(product_a, cents(&price_a)), (product_b, cents(&price_b))]);
    world.stores.push(Box::new(FakeStore { name: store, prices }));
}

#[given(regex = r#"^a store "([^"]+)" selling "([^"]+)" at (\d+\.\d+)$"#)]
fn given_store_one_product(world: &mut BotWorld, store: String, product: String, price: String) {
    let prices = HashMap::from([(product, cents(&price))]);
    world.stores.push(Box::new(FakeStore { name: store, prices }));
}

#[given("a Telegram bot connected to the mock")]
fn given_bot(world: &mut BotWorld) {
    let uri = world.server.as_ref().expect("mock server not started").uri();
    let stores = std::mem::take(&mut world.stores);
    world.bot = Some(TelegramBot::new(uri, BOT_TOKEN.into(), CONFIGURED_CHAT, stores));
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when("the bot processes one round of updates")]
async fn when_run_once(world: &mut BotWorld) {
    world.bot.as_mut().expect("bot not built").run_once().await.expect("run_once failed");
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(regex = r#"^a reply mentioning "([^"]+)" was sent to the configured chat$"#)]
async fn then_reply_sent(world: &mut BotWorld, needle: String) {
    let sends = requests_to(world, "/sendMessage").await;
    let matched = sends.iter().any(|r| {
        let body = String::from_utf8_lossy(&r.body);
        body.contains(&needle) && body.contains(&format!("\"chat_id\":{CONFIGURED_CHAT}"))
    });
    assert!(
        matched,
        "expected a sendMessage to chat {CONFIGURED_CHAT} containing {needle:?}; got: {:?}",
        sends.iter().map(|r| String::from_utf8_lossy(&r.body).to_string()).collect::<Vec<_>>()
    );
}

#[then("no reply was sent")]
async fn then_no_reply(world: &mut BotWorld) {
    let sends = requests_to(world, "/sendMessage").await;
    assert!(
        sends.is_empty(),
        "expected no sendMessage calls, got: {:?}",
        sends.iter().map(|r| String::from_utf8_lossy(&r.body).to_string()).collect::<Vec<_>>()
    );
}

#[then("the next poll asks only for updates after the processed one")]
async fn then_offset_advanced(world: &mut BotWorld) {
    // Poll again and inspect what the bot actually asked the API for.
    world.bot.as_mut().expect("bot not built").run_once().await.expect("run_once failed");
    let expected = format!("offset={}", UPDATE_ID + 1);
    let polls = requests_to(world, "/getUpdates").await;
    let matched = polls
        .iter()
        .any(|r| r.url.query().unwrap_or_default().contains(&expected));
    assert!(
        matched,
        "expected a getUpdates poll with {expected}; got queries: {:?}",
        polls.iter().map(|r| r.url.query().unwrap_or_default().to_string()).collect::<Vec<_>>()
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    BotWorld::run("features/telegram_bot.feature").await;
}
