//! Orchestration: poll Telegram, authorize the sender, interpret the message,
//! run it on the host within a timeout, and reply with the result.
//!
//! `TelegramGateway` is the port to Telegram; the HTTP adapter lives in
//! `http.rs` (added with the wiring). The bot depends only on the ports
//! (`TelegramGateway`, `CommandExecutor`) plus the domain pieces.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::{authorizer::Authorizer, executor::CommandExecutor, formatter, request::Request};

/// One incoming Telegram update reduced to what the bot needs.
#[derive(Clone, Debug)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub chat_id: i64,
    pub text: String,
}

/// Port to Telegram: long-poll for updates and post replies.
#[async_trait]
pub trait TelegramGateway: Send + Sync {
    async fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate>;
    async fn post_message(&self, chat_id: i64, text: &str);
}

const HELP_TEXT: &str =
    "Send a shell command to run it on the host. Reserved commands: /start and /help.";

/// Ties the gateway, authorizer, executor, and formatter together.
pub struct TelegramBot {
    gateway: Arc<dyn TelegramGateway>,
    authorizer: Authorizer,
    executor: Arc<dyn CommandExecutor>,
    timeout: Duration,
}

impl TelegramBot {
    pub fn new(
        gateway: Arc<dyn TelegramGateway>,
        authorizer: Authorizer,
        executor: Arc<dyn CommandExecutor>,
        timeout: Duration,
    ) -> Self {
        Self { gateway, authorizer, executor, timeout }
    }

    /// Fetch the pending updates and handle each, advancing `offset` past them.
    pub async fn run_once(&self, offset: &mut i64) {
        let updates = self.gateway.fetch_updates(*offset).await;
        for update in updates {
            if update.update_id >= *offset {
                *offset = update.update_id + 1;
            }
            self.handle(&update).await;
        }
    }

    async fn handle(&self, update: &TelegramUpdate) {
        if !self.authorizer.is_authorized(update.chat_id) {
            return;
        }
        match Request::parse(&update.text) {
            Request::Ignore => {}
            Request::Help => self.gateway.post_message(update.chat_id, HELP_TEXT).await,
            Request::Run(command) => {
                let reply = self.run_command(&command).await;
                self.gateway.post_message(update.chat_id, &reply).await;
            }
        }
    }

    /// Run a command on the host, applying the timeout and turning every
    /// outcome into a reply string.
    async fn run_command(&self, command: &str) -> String {
        match tokio::time::timeout(self.timeout, self.executor.execute(command)).await {
            Ok(Ok(output)) => formatter::format(&output),
            Ok(Err(e)) => format!("error: {e}"),
            Err(_) => format!("timed out after {}s", self.timeout.as_secs()),
        }
    }

    /// Poll forever, handling updates as they arrive.
    pub async fn run(&self) {
        let mut offset = 0;
        loop {
            self.run_once(&mut offset).await;
        }
    }
}
