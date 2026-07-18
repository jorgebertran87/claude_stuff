use std::collections::HashMap;
use std::sync::Mutex;

use crate::domain::model::game_world::GameSession;
use crate::domain::ports::SessionRepository;

#[derive(Debug)]
pub struct InMemorySessionRepository {
    sessions: Mutex<HashMap<String, GameSession>>,
}

impl InMemorySessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl SessionRepository for InMemorySessionRepository {
    fn save(&self, session: GameSession) {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.id().to_string(), session);
    }

    fn find(&self, id: &str) -> Option<GameSession> {
        self.sessions.lock().unwrap().get(id).cloned()
    }
}
