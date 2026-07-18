use std::fmt::Debug;

use crate::domain::model::game_world::GameSession;

pub trait SessionRepository: Debug + Send + Sync {
    fn save(&self, session: GameSession);
    fn find(&self, id: &str) -> Option<GameSession>;
}
