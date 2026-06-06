//! host_controller — run non-root commands on the parent host over SSH,
//! driven by a Telegram bot.
//!
//! Hexagonal / ports & adapters:
//! - domain (crate root): authorization, request parsing, output formatting
//! - ports: `executor` (run a command) and `telegram` (talk to chat)
//! - adapters: SSH executor, HTTP Telegram gateway
//! - wiring: `main.rs`
//!
//! Modules are added feature-by-feature via /tdd.

pub mod authorizer;
pub mod executor;
pub mod formatter;
pub mod request;
pub mod telegram;
