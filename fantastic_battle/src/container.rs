use std::sync::Arc;

use crate::domain::ports::{BattleRepository, MapRepository, NpcNameGenerator, QuestionAsker, SessionRepository};
use crate::domain::service::{BattleService, GameWorldService};
use crate::infrastructure::map::StaticMapRepository;
use crate::infrastructure::npc::{DeepSeekNpcNameGenerator, StubNpcNameGenerator};
use crate::infrastructure::persistence::{InMemoryBattleRepository, InMemorySessionRepository};
use crate::infrastructure::question::{DeepSeekQuestionAsker, StubQuestionAsker};

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

fn stub_names() -> Vec<String> {
    vec![
        "Sphinx".to_string(),
        "Medusa".to_string(),
        "Minotaur".to_string(),
        "Cyclops".to_string(),
        "Cerberus".to_string(),
    ]
}

pub fn build_state() -> AppState {
    let map_repo: Arc<dyn MapRepository> = Arc::new(StaticMapRepository);
    let session_repo: Arc<dyn SessionRepository> = Arc::new(InMemorySessionRepository::new());
    let battle_repo: Arc<dyn BattleRepository> = Arc::new(InMemoryBattleRepository::new());

    let deepseek_key = std::env::var("DEEPSEEK_API_KEY").ok();

    let (name_generator, question_asker): (
        Arc<dyn NpcNameGenerator>,
        Arc<dyn QuestionAsker>,
    ) = match deepseek_key {
        Some(key) => (
            Arc::new(DeepSeekNpcNameGenerator::new(key.clone())),
            Arc::new(DeepSeekQuestionAsker::new(key)),
        ),
        None => (
            Arc::new(StubNpcNameGenerator::new(stub_names())),
            Arc::new(StubQuestionAsker::new()),
        ),
    };

    AppState {
        game_service: Arc::new(GameWorldService::new(map_repo, session_repo, name_generator)),
        battle_service: Arc::new(BattleService::new(question_asker)),
        battle_repo,
    }
}

pub fn test_state(
    map_repo: Arc<dyn MapRepository>,
    session_repo: Arc<dyn SessionRepository>,
) -> AppState {
    let name_generator: Arc<dyn NpcNameGenerator> = Arc::new(StubNpcNameGenerator::new(vec![
        "Sphinx".to_string(),
        "Medusa".to_string(),
        "Minotaur".to_string(),
    ]));
    let question_asker: Arc<dyn QuestionAsker> = Arc::new(StubQuestionAsker::new());
    let battle_repo: Arc<dyn BattleRepository> = Arc::new(InMemoryBattleRepository::new());
    AppState {
        game_service: Arc::new(GameWorldService::new(map_repo, session_repo, name_generator)),
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

pub fn test_npc_name_generator() -> Box<dyn NpcNameGenerator> {
    Box::new(StubNpcNameGenerator::new(vec![
        "Sphinx".to_string(),
        "Medusa".to_string(),
        "Minotaur".to_string(),
        "Cyclops".to_string(),
        "Cerberus".to_string(),
    ]))
}
