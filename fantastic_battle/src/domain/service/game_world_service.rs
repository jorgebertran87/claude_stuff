use std::sync::Arc;

use crate::domain::model::game_world::{Direction, GameSession, MoveError, Npc, Position};
use crate::domain::model::Theme;
use crate::domain::ports::{MapRepository, NpcNameGenerator, SessionRepository};

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
    name_generator: Arc<dyn NpcNameGenerator>,
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
        name_generator: Arc<dyn NpcNameGenerator>,
    ) -> Self {
        Self {
            map_repo,
            session_repo,
            name_generator,
        }
    }

    pub fn join(&self, theme: &Theme) -> GameSession {
        let mut map = self.map_repo.load();
        let names = self
            .name_generator
            .generate(theme, map.npc_spawns.len() as u32);
        for (spawn, name) in map.npc_spawns.iter_mut().zip(names.iter()) {
            spawn.name = name.name().to_string();
        }
        let session = GameSession::new(map, theme.clone());
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
