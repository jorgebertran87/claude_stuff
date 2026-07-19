use crate::domain::model::{Player, Theme};
use crate::domain::ports::NpcNameGenerator;

#[derive(Debug)]
pub struct DeepSeekNpcNameGenerator {
    api_key: String,
    client: reqwest::blocking::Client,
}

impl DeepSeekNpcNameGenerator {
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
            "max_tokens": 100,
            "temperature": 0.9
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

    fn parse_names(response: &str, count: u32) -> Vec<String> {
        response
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| l.trim_start_matches(|c: char| c == '-' || c == '.' || c.is_ascii_digit()).trim().to_string())
            .filter(|l| !l.is_empty())
            .take(count as usize)
            .collect()
    }
}

impl NpcNameGenerator for DeepSeekNpcNameGenerator {
    fn generate(&self, theme: &Theme, count: u32) -> Vec<Player> {
        let prompt = format!(
            "Generate {count} names of famous people or characters associated with the theme '{theme}'. Return ONLY the names, one per line. No numbering, no dashes, no extra text.",
            count = count,
            theme = theme.value()
        );

        let names = self
            .ask_llm(&prompt)
            .as_deref()
            .map(|r| Self::parse_names(r, count))
            .unwrap_or_default();

        let filled: Vec<Player> = (0..count)
            .map(|i| {
                names
                    .get(i as usize)
                    .and_then(|n| Player::new(n).ok())
                    .unwrap_or_else(|| {
                        Player::new(&format!("{} Expert {}", theme.value(), i + 1)).unwrap()
                    })
            })
            .collect();

        filled
    }
}
