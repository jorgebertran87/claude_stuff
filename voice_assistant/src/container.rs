//! Dependency Injection container.
//! Defines the application module and wires all components to their abstractions.
//! This is the only place in the codebase that names concrete infrastructure types.

use std::path::PathBuf;
use std::sync::Arc;

use shaku::{module, HasComponent, Interface};

use crate::domain::ports::{
    GoogleSheetsGateway, ImageAnalyzer, OrderHandler, TextSynthesizer,
};
use crate::infrastructure::{
    claude_handler::{ClaudeBackend, ClaudeCliBackend, ClaudeCodeHandler,
                     ClaudeCodeHandlerParameters, ClaudeImageAnalyzer},
    google_sheets::GoogleSheetsGatewayImpl,
    speaker::GttsTextSynthesizer,
    telegram_bot::{TelegramBot, TelegramGateway, UreqGateway, UreqGatewayParameters},
};

// ── Interface marker impls ────────────────────────────────────────────────────

impl Interface for dyn OrderHandler {}
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

            // ── text-to-speech ────────────────────────────────────────────────
            GttsTextSynthesizer,  // → TextSynthesizer

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
            log_file:   PathBuf::from(".orders_tokens"),
            session_id: Default::default(),
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

pub fn make_order_handler() -> Arc<dyn OrderHandler> {
    Arc::new(ClaudeCodeHandler::new())
}
