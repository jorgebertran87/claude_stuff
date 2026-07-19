use std::collections::HashMap;
use std::sync::Mutex;

use crate::domain::model::Battle;
use crate::domain::ports::BattleRepository;

#[derive(Debug)]
pub struct InMemoryBattleRepository {
    battles: Mutex<HashMap<String, Battle>>,
}

impl InMemoryBattleRepository {
    pub fn new() -> Self {
        Self {
            battles: Mutex::new(HashMap::new()),
        }
    }
}

impl BattleRepository for InMemoryBattleRepository {
    fn save(&self, session_id: &str, battle: Battle) {
        self.battles
            .lock()
            .unwrap()
            .insert(session_id.to_string(), battle);
    }

    fn find(&self, session_id: &str) -> Option<Battle> {
        self.battles.lock().unwrap().get(session_id).cloned()
    }
}
