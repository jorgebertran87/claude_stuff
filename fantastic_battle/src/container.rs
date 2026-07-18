use std::sync::Arc;

use crate::domain::ports::{MapRepository, SessionRepository};
use crate::domain::service::GameWorldService;
use crate::infrastructure::map::StaticMapRepository;
use crate::infrastructure::persistence::InMemorySessionRepository;

#[derive(Clone, Debug)]
pub struct AppState {
    pub service: Arc<GameWorldService>,
}

pub fn build_state() -> AppState {
    let map_repo: Arc<dyn MapRepository> = Arc::new(StaticMapRepository);
    let session_repo: Arc<dyn SessionRepository> = Arc::new(InMemorySessionRepository::new());
    AppState {
        service: Arc::new(GameWorldService::new(map_repo, session_repo)),
    }
}

pub fn test_state(
    map_repo: Arc<dyn MapRepository>,
    session_repo: Arc<dyn SessionRepository>,
) -> AppState {
    AppState {
        service: Arc::new(GameWorldService::new(map_repo, session_repo)),
    }
}
