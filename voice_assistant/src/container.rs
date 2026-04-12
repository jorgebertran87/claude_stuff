//! Dependency Injection container.
//! Defines the application module and wires all components to their abstractions.
//! This is the only place in the codebase that names concrete infrastructure types.

use std::path::PathBuf;
use std::sync::Arc;

use shaku::{module, HasComponent, Interface};

use crate::domain::model::{Language, WakeWord};
use crate::domain::ports::{
    AudioCapturer, AudioSpeaker, GoogleSheetsGateway, ImageAnalyzer, OrderHandler, TextSynthesizer,
    Transcriber,
};
use crate::domain::service::VoiceListenerService;
use crate::infrastructure::{
    audio::MicrophoneCapturer,
    claude_handler::{ClaudeBackend, ClaudeCliBackend, ClaudeCodeHandler,
                     ClaudeCodeHandlerParameters, ClaudeImageAnalyzer},
    google_sheets::GoogleSheetsGatewayImpl,
    speaker::{GTTSSpeaker, GttsTextSynthesizer},
    telegram_bot::{TelegramBot, TelegramGateway, UreqGateway, UreqGatewayParameters},
    transcriber::GoogleTranscriber,
};

// ── Interface marker impls ────────────────────────────────────────────────────
// shaku's blanket impl only covers Sized types; we must register each dyn Trait
// explicitly. These impls live here to keep the domain free of shaku knowledge.

impl Interface for dyn AudioCapturer {}
impl Interface for dyn OrderHandler {}
impl Interface for dyn Transcriber {}
impl Interface for dyn AudioSpeaker {}
impl Interface for dyn GoogleSheetsGateway {}
impl Interface for dyn TextSynthesizer {}
impl Interface for dyn ImageAnalyzer {}
impl Interface for dyn ClaudeBackend {}
impl Interface for dyn TelegramGateway {}

// ── Application module ────────────────────────────────────────────────────────

module! {
    pub AppModule {
        components = [
            // ── claude ────────────────────────────────────────────────────────
            ClaudeCliBackend,     // → ClaudeBackend
            ClaudeCodeHandler,    // → OrderHandler  (injects ClaudeBackend)
            ClaudeImageAnalyzer,  // → ImageAnalyzer

            // ── google ────────────────────────────────────────────────────────
            GoogleSheetsGatewayImpl,  // → GoogleSheetsGateway
            GoogleTranscriber,        // → Transcriber

            // ── audio / speech ────────────────────────────────────────────────
            MicrophoneCapturer,   // → AudioCapturer
            GttsTextSynthesizer,  // → TextSynthesizer
            GTTSSpeaker,          // → AudioSpeaker

            // ── telegram ─────────────────────────────────────────────────────
            UreqGateway,          // → TelegramGateway  (token parameter)
        ],
        providers = []
    }
}

// ── Module builder ────────────────────────────────────────────────────────────

fn build_module(telegram_token: String) -> AppModule {
    AppModule::builder()
        .with_component_parameters::<ClaudeCodeHandler>(ClaudeCodeHandlerParameters {
            log_file: PathBuf::from(".orders_tokens"),
            ..Default::default()
        })
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
        allowed,
    )
}

/// Create a fresh `OrderHandler`. Used both for direct-order (CLI) mode and as
/// a factory passed to the Telegram bot (one handler per chat session).
pub fn make_order_handler() -> Arc<dyn OrderHandler> {
    Arc::new(ClaudeCodeHandler::new())
}

/// Build a ready-to-use `VoiceListenerService`.
pub fn build_voice_service(wake_word: WakeWord, language: Language) -> VoiceListenerService {
    let module = build_module(String::new());
    VoiceListenerService::new(
        HasComponent::<dyn AudioCapturer>::resolve(&module),
        HasComponent::<dyn Transcriber>::resolve(&module),
        HasComponent::<dyn OrderHandler>::resolve(&module),
        HasComponent::<dyn AudioSpeaker>::resolve(&module),
        wake_word,
        language,
    )
}

