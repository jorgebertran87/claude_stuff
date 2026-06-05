/// A control action requested through a per-action command (`/start`, `/stop`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Start,
    Stop,
    Restart,
    Status,
}

impl Action {
    /// The command word that selects this action (e.g. `/start`).
    pub fn command(&self) -> &'static str {
        match self {
            Action::Start => "/start",
            Action::Stop => "/stop",
            Action::Restart => "/restart",
            Action::Status => "/status",
        }
    }
}

/// A parsed service command: an action plus the alias it targets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceCommand {
    pub action: Action,
    pub alias: String,
}

/// Why a piece of text is not a runnable service command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Not one of our commands (plain chatter or an unrelated command) — ignore it.
    NotACommand,
    /// A recognised action with no alias argument — reply with usage.
    MissingAlias(Action),
}

impl ServiceCommand {
    /// Parse a raw message into a command. Pure domain logic — no I/O.
    ///
    /// Recognises `/start`, `/stop`, `/restart`, `/status` (optionally suffixed
    /// with `@botname` as Telegram does in groups), each taking an alias.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut parts = text.split_whitespace();
        let word = parts.next().ok_or(ParseError::NotACommand)?;
        // Strip the optional "@botname" suffix added in group chats.
        let word = word.split('@').next().unwrap_or(word);

        let action = match word {
            "/start" => Action::Start,
            "/stop" => Action::Stop,
            "/restart" => Action::Restart,
            "/status" => Action::Status,
            _ => return Err(ParseError::NotACommand),
        };

        let alias = parts.next().ok_or(ParseError::MissingAlias(action))?;
        Ok(Self { action, alias: alias.to_string() })
    }
}
