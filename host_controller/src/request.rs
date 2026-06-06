//! Domain: interpret an incoming Telegram message.
//!
//! Plain text is a command to run on the host. The reserved `/start` and
//! `/help` commands are meta-actions (show usage) and never reach the host.
//! A blank message is ignored. A `@botname` suffix on the command word
//! (Telegram's group form) is stripped before matching.

#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    /// `/start` or `/help` — show usage; never executed on the host.
    Help,
    /// Run this (trimmed) command on the host.
    Run(String),
    /// Nothing actionable (blank/whitespace-only message).
    Ignore,
}

impl Request {
    pub fn parse(text: &str) -> Request {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Request::Ignore;
        }
        let first = trimmed.split_whitespace().next().unwrap_or("");
        let command_word = first.split('@').next().unwrap_or(first);
        match command_word {
            "/start" | "/help" => Request::Help,
            _ => Request::Run(trimmed.to_string()),
        }
    }
}
