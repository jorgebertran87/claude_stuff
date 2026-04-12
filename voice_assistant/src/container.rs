//! Dependency Injection container.
//! Defines the application module and wires all components to their abstractions.

use std::path::PathBuf;

use shaku::{module, Interface};

use crate::domain::ports::{
    AudioSpeaker, GoogleSheetsGateway, ImageAnalyzer, OrderHandler, TextSynthesizer, Transcriber,
};
use crate::infrastructure::{
    claude_handler::{ClaudeBackend, ClaudeCliBackend, ClaudeCodeHandler,
                     ClaudeCodeHandlerParameters, ClaudeImageAnalyzer},
    google_sheets::GoogleSheetsGatewayImpl,
    speaker::{GTTSSpeaker, GttsTextSynthesizer},
    telegram_bot::{TelegramGateway, UreqGateway, UreqGatewayParameters},
    transcriber::GoogleTranscriber,
};

// ── Interface marker impls ────────────────────────────────────────────────────
// shaku's blanket impl only covers Sized types; we must register each dyn Trait
// explicitly. These impls live here to keep the domain free of shaku knowledge.

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
            GttsTextSynthesizer,  // → TextSynthesizer
            GTTSSpeaker,          // → AudioSpeaker

            // ── telegram ─────────────────────────────────────────────────────
            UreqGateway,          // → TelegramGateway  (token parameter)
        ],
        providers = []
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build the DI container.
///
/// - `telegram_token`: Telegram Bot API token. Pass an empty string for voice-only mode.
/// - Token log file is always `.orders_tokens`.
pub fn build_module(telegram_token: String) -> AppModule {
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
