use std::sync::Arc;

use crate::domain::model::game_world::{
    Direction, GameSession, MoveError, Npc, NpcSpawn, Position, TileType,
};
use crate::domain::model::{BattleOutcome, Theme};
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

    pub fn join(&self, theme: &Theme, question_count: u32) -> GameSession {
        let mut map = self.map_repo.load();
        let names = self.name_generator.generate(theme, question_count);
        let count = question_count as usize;
        let spawn_count = map.npc_spawns.len();

        if count <= spawn_count {
            map.npc_spawns.truncate(count);
        } else {
            let extra = count - spawn_count;
            let occupied: std::collections::HashSet<Position> = map
                .npc_spawns
                .iter()
                .map(|s| s.position)
                .chain(std::iter::once(map.start_position))
                .collect();
            let mut candidates: Vec<Position> = map
                .tiles
                .iter()
                .filter(|(pos, tile)| {
                    **tile == TileType::Grass && !occupied.contains(pos)
                })
                .map(|(pos, _)| *pos)
                .collect();
            candidates.sort_by(|a, b| a.y().cmp(&b.y()).then_with(|| a.x().cmp(&b.x())));
            for pos in candidates.iter().take(extra) {
                map.npc_spawns.push(NpcSpawn {
                    name: format!("extra-{}", map.npc_spawns.len()),
                    position: *pos,
                    direction: Direction::South,
                });
            }
        }

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

    pub fn defeat_npc(
        &self,
        session_id: &str,
        npc_name: &str,
        outcome: BattleOutcome,
    ) -> Result<(), GameWorldError> {
        let mut session = self
            .session_repo
            .find(session_id)
            .ok_or(GameWorldError::SessionNotFound)?;
        session.defeat_npc_by_name(npc_name, outcome);
        self.session_repo.save(session);
        Ok(())
    }
}
