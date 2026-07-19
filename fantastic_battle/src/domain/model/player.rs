#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerError {
    NameRequired,
}

impl std::fmt::Display for PlayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerError::NameRequired => write!(f, "player name is required"),
        }
    }
}

impl Player {
    pub fn new(name: &str) -> Result<Self, PlayerError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(PlayerError::NameRequired);
        }
        Ok(Self { name: name.to_string() })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
