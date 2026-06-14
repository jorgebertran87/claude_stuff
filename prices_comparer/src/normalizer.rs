use std::io::Write;
use std::process::{Command, Stdio};

use async_trait::async_trait;
use serde::Deserialize;

use crate::basket::{OrderNormalizer, PurchasedBasket, PurchasedItem};

/// Normalizes a purchased basket by asking Claude (via the `claude` CLI) to
/// rewrite store-brand line names into generic, searchable product names,
/// keeping each line's quantity and price.
///
/// Auth comes from the mounted Claude Code credentials; the prompt is the
/// `.claude/commands/normalize_order.md` skill. The CLI is blocking, so it
/// runs on a blocking task. Not exercised by the test suite — callers fall
/// back to the raw items when it errors.
pub struct ClaudeCliNormalizer {
    model: String,
}

impl ClaudeCliNormalizer {
    pub fn new() -> Self {
        Self { model: "claude-haiku-4-5".to_string() }
    }
}

impl Default for ClaudeCliNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

/// The `claude --output-format json` envelope.
#[derive(Deserialize)]
struct CliResult {
    result: String,
}

/// One cleaned line as Claude returns it.
#[derive(Deserialize)]
struct CleanItem {
    name: String,
    #[serde(default = "one")]
    quantity: u64,
    #[serde(default)]
    price_cents: Option<u64>,
}

fn one() -> u64 {
    1
}

#[async_trait]
impl OrderNormalizer for ClaudeCliNormalizer {
    async fn normalize(&self, basket: &PurchasedBasket) -> anyhow::Result<Vec<PurchasedItem>> {
        let input = serde_json::json!({
            "store": basket.store,
            "items": basket
                .items
                .iter()
                .map(|i| serde_json::json!({
                    "name": i.name,
                    "quantity": i.quantity,
                    "price_cents": i.price_cents,
                }))
                .collect::<Vec<_>>(),
        })
        .to_string();
        let prompt = load_skill("normalize_order");
        let model = self.model.clone();

        let output = tokio::task::spawn_blocking(move || {
            let mut child = Command::new("claude")
                .args(["--print", "--output-format", "json", "--model", &model])
                .args(["--system-prompt", &prompt])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input.as_bytes())?;
            }
            child.wait_with_output()
        })
        .await??;

        if !output.status.success() {
            anyhow::bail!("claude exited: {}", String::from_utf8_lossy(&output.stderr));
        }

        let envelope: CliResult = serde_json::from_slice(&output.stdout)?;
        let array = extract_json_array(&envelope.result)
            .ok_or_else(|| anyhow::anyhow!("no JSON array in claude result"))?;
        let clean: Vec<CleanItem> = serde_json::from_str(array)?;
        Ok(carry_sizes_over(clean, basket))
    }
}

/// Combine the model's cleaned lines with the input basket: take the new name,
/// quantity and price from the model, and carry each line's size over from the
/// input by position (the cleaned name has the size stripped out).
fn carry_sizes_over(clean: Vec<CleanItem>, basket: &PurchasedBasket) -> Vec<PurchasedItem> {
    clean
        .into_iter()
        .enumerate()
        .map(|(i, c)| PurchasedItem {
            name: c.name,
            quantity: c.quantity.max(1),
            price_cents: c.price_cents,
            size: basket.items.get(i).and_then(|it| it.size),
        })
        .collect()
}

/// Normalizes a purchased basket by asking DeepSeek (OpenAI-compatible chat
/// completions) to rewrite store-brand line names into generic, searchable
/// product names, keeping each line's quantity and price. The `normalize_order`
/// skill is the system prompt. On any HTTP error or unusable reply it returns
/// an error, so callers fall back to the raw items.
pub struct DeepSeekNormalizer {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl DeepSeekNormalizer {
    /// Talk to DeepSeek's public API with the given key.
    pub fn new(api_key: String) -> Self {
        Self::with_base_url("https://api.deepseek.com".to_string(), api_key)
    }

    /// `base_url` is the DeepSeek host in production or a mock server in tests.
    pub fn with_base_url(base_url: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            model: "deepseek-chat".to_string(),
        }
    }
}

/// The OpenAI-compatible chat completions envelope DeepSeek returns.
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[async_trait]
impl OrderNormalizer for DeepSeekNormalizer {
    async fn normalize(&self, basket: &PurchasedBasket) -> anyhow::Result<Vec<PurchasedItem>> {
        let input = serde_json::json!({
            "store": basket.store,
            "items": basket
                .items
                .iter()
                .map(|i| serde_json::json!({
                    "name": i.name,
                    "quantity": i.quantity,
                    "price_cents": i.price_cents,
                }))
                .collect::<Vec<_>>(),
        })
        .to_string();
        let prompt = load_skill("normalize_order");

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [
                    { "role": "system", "content": prompt },
                    { "role": "user", "content": input },
                ],
            }))
            .send()
            .await?
            .error_for_status()?;

        let chat: ChatResponse = response.json().await?;
        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .ok_or_else(|| anyhow::anyhow!("DeepSeek returned no choices"))?;
        let array = extract_json_array(content)
            .ok_or_else(|| anyhow::anyhow!("no JSON array in DeepSeek reply"))?;
        let clean: Vec<CleanItem> = serde_json::from_str(array)?;
        Ok(carry_sizes_over(clean, basket))
    }
}

/// Extract the first JSON array from text — Claude may wrap it in prose or
/// code fences despite instructions.
fn extract_json_array(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    (end > start).then(|| &text[start..=end])
}

/// Load a skill prompt from `.claude/commands`, stripping YAML frontmatter.
fn load_skill(name: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let candidates = [
        format!("/app/.claude/commands/{name}.md"),
        format!("{home}/.claude/commands/{name}.md"),
        format!(".claude/commands/{name}.md"),
    ];
    for path in candidates {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return strip_frontmatter(&content);
        }
    }
    eprintln!("[normalizer: skill '{name}' not found]");
    String::new()
}

fn strip_frontmatter(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim().to_string();
        }
    }
    content.trim().to_string()
}
