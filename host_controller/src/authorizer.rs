//! Domain gate: decides whether a Telegram chat may run commands.
//!
//! This bot is **DM-only**. Telegram private chats have positive IDs;
//! groups, supergroups, and channels have negative IDs. A chat is authorized
//! only when its ID is positive *and* present in the configured allowlist.
//! An empty allowlist therefore denies everyone — the endpoint fails closed.

use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct Authorizer {
    allowed: HashSet<i64>,
}

impl Authorizer {
    /// Build from the configured chat IDs (e.g. parsed from `TELEGRAM_ALLOWED_CHATS`).
    pub fn new(allowed: impl IntoIterator<Item = i64>) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
        }
    }

    /// True only for a direct (positive) chat ID that is on the allowlist.
    pub fn is_authorized(&self, chat_id: i64) -> bool {
        chat_id > 0 && self.allowed.contains(&chat_id)
    }
}
