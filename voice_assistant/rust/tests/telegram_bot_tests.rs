//! Unit tests for TelegramBot behaviour.
//! Detroit School: hand-rolled fakes, no mock library.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::infrastructure::telegram_bot::{TelegramBot, TelegramGateway, TelegramUpdate};

// ── Fakes ─────────────────────────────────────────────────────────────────────

struct FakeGateway {
    updates: Vec<TelegramUpdate>,
    posted: Arc<Mutex<Vec<(i64, String)>>>,
}

impl FakeGateway {
    fn new(updates: Vec<TelegramUpdate>) -> (Self, Arc<Mutex<Vec<(i64, String)>>>) {
        let posted = Arc::new(Mutex::new(Vec::new()));
        (Self { updates, posted: posted.clone() }, posted)
    }
}

impl TelegramGateway for FakeGateway {
    fn fetch_updates(&self, _offset: i64) -> Vec<TelegramUpdate> {
        self.updates.clone()
    }
    fn post_message(&self, chat_id: i64, text: &str) {
        self.posted.lock().unwrap().push((chat_id, text.to_string()));
    }
    fn send_voice(&self, _chat_id: i64, _data: &[u8]) {}
}

struct FakeHandler {
    response: String,
}

impl OrderHandler for FakeHandler {
    fn handle(&self, _order: &str) -> String { self.response.clone() }
    fn reset_session(&self) {}
}

fn make_bot_with_updates(updates: Vec<TelegramUpdate>) -> (TelegramBot, Arc<Mutex<Vec<(i64, String)>>>) {
    let (gateway, posted) = FakeGateway::new(updates);
    let bot = TelegramBot::with_injectable(Box::new(gateway), vec![]);
    (bot, posted)
}

fn no_speak(_: &str) {}

// ── /voice_mode command ───────────────────────────────────────────────────────

#[test]
fn voice_mode_command_sends_activation_confirmation() {
    let updates = vec![TelegramUpdate { update_id: 1, chat_id: 10, text: "/voice_mode".into() }];
    let (bot, posted) = make_bot_with_updates(updates);
    let mut handlers = HashMap::new();
    let mut voice_mode_chats = HashSet::new();
    let mut offset = 0i64;

    bot.run_once(
        &|| Arc::new(FakeHandler { response: "ok".into() }),
        &mut handlers,
        &mut voice_mode_chats,
        &mut offset,
        &no_speak,
        &|| {},
    );

    let msgs = posted.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], (10, "Modo voz activado.".to_string()));
}

#[test]
fn voice_mode_enabled_speaks_response_via_speak_text() {
    let updates = vec![
        TelegramUpdate { update_id: 1, chat_id: 10, text: "/voice_mode".into() },
        TelegramUpdate { update_id: 2, chat_id: 10, text: "hola".into() },
    ];
    let (bot, _posted) = make_bot_with_updates(updates);
    let mut handlers = HashMap::new();
    let mut voice_mode_chats = HashSet::new();
    let mut offset = 0i64;
    let spoken: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let spoken_clone = spoken.clone();

    bot.run_once(
        &|| Arc::new(FakeHandler { response: "respuesta".into() }),
        &mut handlers,
        &mut voice_mode_chats,
        &mut offset,
        &move |text| spoken_clone.lock().unwrap().push(text.to_string()),
        &|| {},
    );

    let s = spoken.lock().unwrap();
    assert_eq!(s.len(), 1, "expected speak_text called once; got: {s:?}");
    assert_eq!(s[0], "respuesta");
}

#[test]
fn voice_mode_command_second_time_deactivates_and_sends_confirmation() {
    let updates = vec![
        TelegramUpdate { update_id: 1, chat_id: 10, text: "/voice_mode".into() },
        TelegramUpdate { update_id: 2, chat_id: 10, text: "/voice_mode".into() },
    ];
    let (bot, posted) = make_bot_with_updates(updates);
    let mut handlers = HashMap::new();
    let mut voice_mode_chats = HashSet::new();
    let mut offset = 0i64;

    bot.run_once(
        &|| Arc::new(FakeHandler { response: "ok".into() }),
        &mut handlers,
        &mut voice_mode_chats,
        &mut offset,
        &no_speak,
        &|| {},
    );

    let msgs = posted.lock().unwrap();
    assert_eq!(msgs[0].1, "Modo voz activado.");
    assert_eq!(msgs[1].1, "Modo voz desactivado.");
}

#[test]
fn voice_mode_disabled_does_not_speak_response() {
    let updates = vec![
        TelegramUpdate { update_id: 1, chat_id: 10, text: "hola".into() },
    ];
    let (bot, _posted) = make_bot_with_updates(updates);
    let mut handlers = HashMap::new();
    let mut voice_mode_chats = HashSet::new();
    let mut offset = 0i64;
    let spoken: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let spoken_clone = spoken.clone();

    bot.run_once(
        &|| Arc::new(FakeHandler { response: "respuesta".into() }),
        &mut handlers,
        &mut voice_mode_chats,
        &mut offset,
        &move |text| spoken_clone.lock().unwrap().push(text.to_string()),
        &|| {},
    );

    assert!(spoken.lock().unwrap().is_empty(), "speak_text must not be called when voice mode is off");
}

#[test]
fn voice_mode_does_not_speak_alexa_spotify_responses() {
    let updates = vec![
        TelegramUpdate { update_id: 1, chat_id: 10, text: "/voice_mode".into() },
        TelegramUpdate { update_id: 2, chat_id: 10, text: "pon música".into() },
    ];
    let (bot, _posted) = make_bot_with_updates(updates);
    let mut handlers = HashMap::new();
    let mut voice_mode_chats = HashSet::new();
    let mut offset = 0i64;
    let spoken: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let spoken_clone = spoken.clone();

    bot.run_once(
        &|| Arc::new(FakeHandler { response: "Alexa, pon \"Bohemian Rhapsody\" en Spotify".into() }),
        &mut handlers,
        &mut voice_mode_chats,
        &mut offset,
        &move |text| spoken_clone.lock().unwrap().push(text.to_string()),
        &|| {},
    );

    assert!(spoken.lock().unwrap().is_empty(), "speak_text must not be called for alexa+spotify responses");
}
