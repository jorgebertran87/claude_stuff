use super::{Player, Question};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleOutcome {
    Victory,
    Defeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleError {
    AlreadyOver,
}

#[derive(Debug, Clone)]
pub struct Battle {
    opponent: Player,
    question: Question,
    outcome: Option<BattleOutcome>,
}

impl Battle {
    pub fn new(opponent: Player, question: Question) -> Self {
        Self {
            opponent,
            question,
            outcome: None,
        }
    }

    pub fn opponent(&self) -> &Player {
        &self.opponent
    }

    pub fn question(&self) -> &Question {
        &self.question
    }

    pub fn outcome(&self) -> Option<BattleOutcome> {
        self.outcome
    }

    pub fn answer(&mut self, answer: &str) -> Result<BattleOutcome, BattleError> {
        if self.outcome.is_some() {
            return Err(BattleError::AlreadyOver);
        }
        let outcome = if self.question.is_correct(answer) {
            BattleOutcome::Victory
        } else {
            BattleOutcome::Defeat
        };
        self.outcome = Some(outcome);
        Ok(outcome)
    }
}
