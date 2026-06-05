pub mod http;

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    command::{Action, ParseError, ServiceCommand},
    manager::ServiceManager,
};

/// A single inbound Telegram message, reduced to what the bot needs.
#[derive(Clone, Debug)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub chat_id: i64,
    pub text: String,
}

/// The port through which the bot talks to Telegram.
///
/// The production adapter ([`http::HttpTelegramGateway`]) speaks the Telegram
/// Bot API over HTTP; tests use an in-memory fake. The bot logic never knows
/// which is in use — Telegram transport is just an adapter.
#[async_trait]
pub trait TelegramGateway: Send + Sync {
    /// Long-poll for new updates starting at `offset`.
    async fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate>;
    /// Post a reply to a chat.
    async fn post_message(&self, chat_id: i64, text: &str);
}

/// Drives service control from Telegram commands.
///
/// Fetches updates from the gateway, authorizes the chat, parses the command
/// (pure domain logic in [`ServiceCommand`]), dispatches through the
/// [`ServiceManager`], and posts the outcome back.
pub struct TelegramBot {
    gateway: Arc<dyn TelegramGateway>,
    manager: Arc<ServiceManager>,
    /// Chats allowed to control services. Empty means "allow any chat".
    allowed_chats: Vec<i64>,
}

impl TelegramBot {
    pub fn new(
        gateway: Arc<dyn TelegramGateway>,
        manager: Arc<ServiceManager>,
        allowed_chats: Vec<i64>,
    ) -> Self {
        Self { gateway, manager, allowed_chats }
    }

    fn is_authorized(&self, chat_id: i64) -> bool {
        self.allowed_chats.is_empty() || self.allowed_chats.contains(&chat_id)
    }

    /// Process one batch of updates, advancing `offset` past them.
    pub async fn run_once(&self, offset: &mut i64) {
        let updates = self.gateway.fetch_updates(*offset).await;
        for update in updates {
            *offset = update.update_id + 1;
            if !self.is_authorized(update.chat_id) {
                continue;
            }
            if let Some(reply) = self.handle_text(&update.text).await {
                self.gateway.post_message(update.chat_id, &reply).await;
            }
        }
    }

    /// Production loop: poll forever, processing each batch.
    pub async fn run(self) {
        let mut offset = 0;
        loop {
            self.run_once(&mut offset).await;
        }
    }

    /// Turn a raw message into an optional reply.
    /// `None` means "stay silent" (not one of our commands).
    async fn handle_text(&self, text: &str) -> Option<String> {
        match ServiceCommand::parse(text) {
            Ok(cmd) => Some(self.dispatch(cmd).await),
            Err(ParseError::NotACommand) => None,
            Err(ParseError::MissingAlias(action)) => {
                Some(format!("Usage: {} <alias>", action.command()))
            }
        }
    }

    async fn dispatch(&self, cmd: ServiceCommand) -> String {
        let alias = &cmd.alias;
        match cmd.action {
            Action::Start => match self.manager.start(alias).await {
                Ok(()) => format!("✅ Service \"{alias}\" started."),
                Err(e) => format!("❌ {e}"),
            },
            Action::Stop => match self.manager.stop(alias).await {
                Ok(()) => format!("✅ Service \"{alias}\" stopped."),
                Err(e) => format!("❌ {e}"),
            },
            Action::Restart => match self.manager.restart(alias).await {
                Ok(()) => format!("✅ Service \"{alias}\" restarted."),
                Err(e) => format!("❌ {e}"),
            },
            Action::Status => match self.manager.status(alias).await {
                Ok(status) => format!("ℹ️ Service \"{alias}\" is {status}."),
                Err(e) => format!("❌ {e}"),
            },
        }
    }
}
