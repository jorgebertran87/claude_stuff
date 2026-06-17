//! Dependency Injection container.
//! Defines the application module and wires all components to their abstractions.
//! This is the only place in the codebase that names concrete infrastructure types.

use std::path::PathBuf;
use std::sync::Arc;

use shaku::{module, HasComponent, Interface};

use crate::domain::model::{Language, WakeWord};
use crate::domain::ports::{
    AudioCapturer, AudioPlayer, AudioSpeaker, GoogleSheetsGateway, ImageAnalyzer,
    MinesweeperAnalyzer, OrderHandler, SkillCommands, TextSynthesizer, Transcriber,
};
use crate::domain::service::VoiceListenerService;
use crate::infrastructure::{
    audio::MicrophoneCapturer,
    audio_player::FfplayAudioPlayer,
    claude_handler::{ClaudeBackend, ClaudeCodeHandler, ClaudeImageAnalyzer, DeepSeekBackend},
    google_sheets::GoogleSheetsGatewayImpl,
    minesweeper::MinesweeperService,
    speaker::{PiperSpeaker, PiperTextSynthesizer},
    telegram_bot::{TelegramBot, TelegramGateway, UreqGateway, UreqGatewayParameters},
    telegram_skills::ClaudeSkillCommands,
    transcriber::GoogleTranscriber,
};

// ── Interface marker impls ────────────────────────────────────────────────────

impl Interface for dyn AudioCapturer {}
impl Interface for dyn Transcriber {}
impl Interface for dyn AudioSpeaker {}
impl Interface for dyn GoogleSheetsGateway {}
impl Interface for dyn TextSynthesizer {}
impl Interface for dyn ImageAnalyzer {}
impl Interface for dyn MinesweeperAnalyzer {}
impl Interface for dyn SkillCommands {}
impl Interface for dyn AudioPlayer {}
impl Interface for dyn TelegramGateway {}

// ── Application module ────────────────────────────────────────────────────────

module! {
    pub AppModule {
        components = [
            // ── claude ────────────────────────────────────────────────────────
            ClaudeImageAnalyzer,  // → ImageAnalyzer

            // ── google ────────────────────────────────────────────────────────
            GoogleSheetsGatewayImpl,  // → GoogleSheetsGateway
            GoogleTranscriber,        // → Transcriber

            // ── audio / speech ────────────────────────────────────────────────
            MicrophoneCapturer,   // → AudioCapturer
            PiperTextSynthesizer,  // → TextSynthesizer
            PiperSpeaker,          // → AudioSpeaker
            FfplayAudioPlayer,    // → AudioPlayer

            // ── telegram ─────────────────────────────────────────────────────
            UreqGateway,          // → TelegramGateway  (token parameter)
            MinesweeperService,   // → MinesweeperAnalyzer
            ClaudeSkillCommands,  // → SkillCommands  (injects GoogleSheetsGateway)
        ],
        providers = []
    }
}

// ── Module builder ────────────────────────────────────────────────────────────

fn build_module(telegram_token: String) -> AppModule {
    AppModule::builder()
        .with_component_parameters::<UreqGateway>(UreqGatewayParameters {
            token: telegram_token,
        })
        .build()
}

// ── Public composition API ────────────────────────────────────────────────────

/// Build a ready-to-use `TelegramBot`. Reads `TELEGRAM_ALLOWED_CHAT_IDS` from env.
pub fn build_telegram_bot(token: String) -> TelegramBot {
    let allowed: Vec<i64> = std::env::var("TELEGRAM_ALLOWED_CHAT_IDS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect();

    let module = build_module(token);
    TelegramBot::with_injectable(
        HasComponent::<dyn TelegramGateway>::resolve(&module),
        HasComponent::<dyn GoogleSheetsGateway>::resolve(&module),
        HasComponent::<dyn TextSynthesizer>::resolve(&module),
        HasComponent::<dyn ImageAnalyzer>::resolve(&module),
        HasComponent::<dyn MinesweeperAnalyzer>::resolve(&module),
        HasComponent::<dyn SkillCommands>::resolve(&module),
        HasComponent::<dyn AudioPlayer>::resolve(&module),
        allowed,
    )
}

/// Create a fresh `OrderHandler` backed by DeepSeek. Used both for
/// direct-order (CLI) mode and as a factory passed to the Telegram bot
/// (one handler per chat session).
pub fn make_order_handler() -> Arc<dyn OrderHandler> {
    Arc::new(ClaudeCodeHandler::new(
        Arc::new(DeepSeekBackend::new()),
        PathBuf::from(".orders_tokens"),
    ))
}

/// Build a ready-to-use `VoiceListenerService` with a DeepSeek-backed order handler.
pub fn build_voice_service(wake_word: WakeWord, language: Language) -> VoiceListenerService {
    let module = build_module(String::new());
    VoiceListenerService::new(
        HasComponent::<dyn AudioCapturer>::resolve(&module),
        HasComponent::<dyn Transcriber>::resolve(&module),
        make_order_handler(),
        HasComponent::<dyn AudioSpeaker>::resolve(&module),
        wake_word,
        language,
    )
}
