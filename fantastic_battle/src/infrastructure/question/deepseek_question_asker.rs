use crate::domain::model::{Player, Question, Theme};
use crate::domain::ports::QuestionAsker;

#[derive(Debug)]
pub struct DeepSeekQuestionAsker {
    api_key: String,
    client: reqwest::blocking::Client,
}

impl DeepSeekQuestionAsker {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn ask_llm(&self, prompt: &str) -> Option<String> {
        let body = serde_json::json!({
            "model": "deepseek-chat",
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "max_tokens": 150,
            "temperature": 0.7
        });

        let response = self
            .client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .ok()?;

        let json: serde_json::Value = response.json().ok()?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
    }

    fn parse_qa(response: &str) -> Option<Question> {
        let mut question_text = None;
        let mut answer_text = None;
        for line in response.lines() {
            let trimmed = line.trim();
            if let Some(q) = trimmed.strip_prefix("Q:").or_else(|| trimmed.strip_prefix("Q: ")) {
                question_text = Some(q.trim().to_string());
            } else if let Some(a) = trimmed.strip_prefix("A:").or_else(|| trimmed.strip_prefix("A: ")) {
                answer_text = Some(a.trim().to_string());
            }
        }
        match (question_text, answer_text) {
            (Some(q), Some(a)) if !q.is_empty() && !a.is_empty() => Some(Question::new(&q, &a)),
            _ => None,
        }
    }
}

impl QuestionAsker for DeepSeekQuestionAsker {
    fn ask(&self, theme: &Theme, player: &Player) -> Question {
        let prompt = format!(
            "You are a trivia game opponent named {name} challenging a player on the topic: {theme}. Ask exactly ONE trivia question with a single, unambiguous, factual answer. The answer must be a short phrase (max 5 words).\n\nFormat your response exactly like this:\nQ: <your question>\nA: <the single correct answer>",
            name = player.name(),
            theme = theme.value()
        );

        self.ask_llm(&prompt)
            .as_deref()
            .and_then(Self::parse_qa)
            .unwrap_or_else(|| {
                Question::new(
                    &format!("What is a notable fact about {}?", theme.value()),
                    theme.value(),
                )
            })
    }
}
