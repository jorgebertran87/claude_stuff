use crate::domain::model::Battle;

pub trait BattleRepository: Send + Sync + std::fmt::Debug {
    fn save(&self, session_id: &str, battle: Battle);
    fn find(&self, session_id: &str) -> Option<Battle>;
}
