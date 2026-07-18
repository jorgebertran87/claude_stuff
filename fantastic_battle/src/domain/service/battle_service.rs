use std::sync::Arc;

use crate::domain::model::{Battle, Player, Theme};
use crate::domain::ports::QuestionAsker;

pub struct BattleService {
    question_asker: Arc<dyn QuestionAsker>,
}

impl BattleService {
    pub fn new(question_asker: Arc<dyn QuestionAsker>) -> Self {
        Self { question_asker }
    }

    pub fn start_battle(&self, theme: &Theme, opponent: &Player) -> Battle {
        let question = self.question_asker.ask(theme, opponent);
        Battle::new(opponent.clone(), question)
    }
}
