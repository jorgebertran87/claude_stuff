//! Dependency Injection container.
//! Defines the application module and wires all components to their abstractions.
//! This is the only place in the codebase that names concrete infrastructure types.

use std::path::PathBuf;
use std::sync::Arc;

use shaku::{module, HasComponent, Interface};

use crate::domain::model::{Language, WakeWord};
use crate::domain::ports::{
    AudioCapturer, AudioPlayer, AudioSpeaker,
    MinesweeperAnalyzer, OrderHandler, SkillCommands, TextSynthesizer, Transcriber,
};
use crate::domain::service::VoiceListenerService;
use crate::infrastructure::{
    audio_capturer::cpal::MicrophoneCapturer,
    audio_player::rodio::RodioAudioPlayer,
    transcriber::google_stt::GoogleTranscriber,
    order_handler::claude::{ClaudeCodeHandler, DeepSeekBackend, ToolOrchestrator},
    minesweeper_analyzer::server::MinesweeperService,
    telegram::{
        telegram_bot::{TelegramBot, TelegramGateway, UreqGateway, UreqGatewayParameters},
    },
    commands::telegram::ClaudeSkillCommands,
    audio_speaker::piper::PiperSpeaker,
    text_synthesizer::piper::PiperTextSynthesizer,
};

// ── Interface marker impls ────────────────────────────────────────────────────

impl Interface for dyn AudioCapturer {}
impl Interface for dyn Transcriber {}
impl Interface for dyn AudioSpeaker {}
impl Interface for dyn TextSynthesizer {}
impl Interface for dyn MinesweeperAnalyzer {}
impl Interface for dyn SkillCommands {}
impl Interface for dyn AudioPlayer {}
impl Interface for dyn TelegramGateway {}

// ── Application module ────────────────────────────────────────────────────────

module! {
    pub AppModule {
        components = [
            // ── google ────────────────────────────────────────────────────────
            GoogleTranscriber,          // → Transcriber

            // ── audio / speech ────────────────────────────────────────────────
            MicrophoneCapturer,   // → AudioCapturer
            PiperTextSynthesizer,  // → TextSynthesizer
            PiperSpeaker,          // → AudioSpeaker
            RodioAudioPlayer,      // → AudioPlayer

            // ── telegram ─────────────────────────────────────────────────────
            UreqGateway,          // → TelegramGateway  (token parameter)
            MinesweeperService,   // → MinesweeperAnalyzer
            ClaudeSkillCommands,  // → SkillCommands
        ],
        providers = []
    }
}

// ── Module builder ────────────────────────────────────────────────────────────

/// Build a module for integration tests (empty Telegram token).
/// Tests resolve adapters through their ports — never import concrete types.
pub fn test_module() -> AppModule {
    build_module(String::new())
}

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
        HasComponent::<dyn TextSynthesizer>::resolve(&module),
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
    let backend = DeepSeekBackend::new()
        .with_tools(Box::new(ToolOrchestrator::new()));
    Arc::new(ClaudeCodeHandler::new(
        Arc::new(backend),
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
