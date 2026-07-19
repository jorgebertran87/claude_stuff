use std::sync::Arc;

use crate::domain::ports::{BattleRepository, MapRepository, QuestionAsker, SessionRepository};
use crate::domain::service::{BattleService, GameWorldService};
use crate::infrastructure::map::StaticMapRepository;
use crate::infrastructure::persistence::{InMemoryBattleRepository, InMemorySessionRepository};
use crate::infrastructure::question::StubQuestionAsker;

#[derive(Clone)]
pub struct AppState {
    pub game_service: Arc<GameWorldService>,
    pub battle_service: Arc<BattleService>,
    pub battle_repo: Arc<dyn BattleRepository>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish()
    }
}

pub fn build_state() -> AppState {
    let map_repo: Arc<dyn MapRepository> = Arc::new(StaticMapRepository);
    let session_repo: Arc<dyn SessionRepository> = Arc::new(InMemorySessionRepository::new());
    let question_asker: Arc<dyn QuestionAsker> = Arc::new(StubQuestionAsker::new());
    let battle_repo: Arc<dyn BattleRepository> = Arc::new(InMemoryBattleRepository::new());
    AppState {
        game_service: Arc::new(GameWorldService::new(map_repo, session_repo)),
        battle_service: Arc::new(BattleService::new(question_asker)),
        battle_repo,
    }
}

pub fn test_state(
    map_repo: Arc<dyn MapRepository>,
    session_repo: Arc<dyn SessionRepository>,
) -> AppState {
    let question_asker: Arc<dyn QuestionAsker> = Arc::new(StubQuestionAsker::new());
    let battle_repo: Arc<dyn BattleRepository> = Arc::new(InMemoryBattleRepository::new());
    AppState {
        game_service: Arc::new(GameWorldService::new(map_repo, session_repo)),
        battle_service: Arc::new(BattleService::new(question_asker)),
        battle_repo,
    }
}

pub fn test_question_asker() -> Box<dyn crate::domain::ports::QuestionAsker> {
    Box::new(crate::infrastructure::question::StubQuestionAsker::new())
}

pub fn test_battle_repository() -> Box<dyn crate::domain::ports::BattleRepository> {
    Box::new(crate::infrastructure::persistence::InMemoryBattleRepository::new())
}
