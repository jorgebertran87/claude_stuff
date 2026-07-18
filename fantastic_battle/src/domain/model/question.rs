#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    text: String,
    correct_answer: String,
}

impl Question {
    pub fn new(text: &str, correct_answer: &str) -> Self {
        Self {
            text: text.to_string(),
            correct_answer: correct_answer.to_string(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_correct(&self, answer: &str) -> bool {
        answer.trim().to_lowercase() == self.correct_answer.trim().to_lowercase()
    }
}
