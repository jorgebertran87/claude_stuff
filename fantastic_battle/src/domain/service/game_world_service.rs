use std::sync::Arc;

use crate::domain::model::game_world::{Direction, GameSession, MoveError, Npc, Position};
use crate::domain::ports::{MapRepository, SessionRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameWorldError {
    SessionNotFound,
    Move(MoveError),
}

impl std::fmt::Display for GameWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameWorldError::SessionNotFound => write!(f, "session not found"),
            GameWorldError::Move(e) => write!(f, "{}", e),
        }
    }
}

pub struct GameWorldService {
    map_repo: Arc<dyn MapRepository>,
    session_repo: Arc<dyn SessionRepository>,
}

impl std::fmt::Debug for GameWorldService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameWorldService").finish()
    }
}

impl GameWorldService {
    pub fn new(
        map_repo: Arc<dyn MapRepository>,
        session_repo: Arc<dyn SessionRepository>,
    ) -> Self {
        Self {
            map_repo,
            session_repo,
        }
    }

    pub fn join(&self) -> GameSession {
        let map = self.map_repo.load();
        let session = GameSession::new(map);
        self.session_repo.save(session.clone());
        session
    }

    pub fn move_player(
        &self,
        session_id: &str,
        direction: Direction,
    ) -> Result<Position, GameWorldError> {
        let mut session = self
            .session_repo
            .find(session_id)
            .ok_or(GameWorldError::SessionNotFound)?;
        let result = session.move_player(direction).map_err(GameWorldError::Move);
        if result.is_ok() {
            self.session_repo.save(session);
        }
        result
    }

    pub fn interact(&self, session_id: &str) -> Result<Option<Npc>, GameWorldError> {
        let session = self
            .session_repo
            .find(session_id)
            .ok_or(GameWorldError::SessionNotFound)?;
        Ok(session.interact().cloned())
    }

    pub fn get_session(&self, session_id: &str) -> Option<GameSession> {
        self.session_repo.find(session_id)
    }
}
