use crate::domain::model::{Player, Question, Theme};

pub trait QuestionAsker: Send + Sync {
    fn ask(&self, theme: &Theme, player: &Player) -> Question;
}
