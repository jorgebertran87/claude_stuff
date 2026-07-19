use std::collections::HashMap;

use crate::domain::model::{Player, Question, Theme};
use crate::domain::ports::QuestionAsker;

#[derive(Debug)]
pub struct StubQuestionAsker {
    questions: HashMap<String, Question>,
}

impl StubQuestionAsker {
    pub fn new() -> Self {
        let mut questions = HashMap::new();
        questions.insert(
            "Sphinx".to_string(),
            Question::new("Who rules Mount Olympus?", "Zeus"),
        );
        questions.insert(
            "Minotaur".to_string(),
            Question::new("Who built the labyrinth?", "Daedalus"),
        );
        Self { questions }
    }
}

impl QuestionAsker for StubQuestionAsker {
    fn ask(&self, _theme: &Theme, player: &Player) -> Question {
        self.questions
            .get(player.name())
            .cloned()
            .unwrap_or_else(|| Question::new("What is the answer?", "42"))
    }
}
