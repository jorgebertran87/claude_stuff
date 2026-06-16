use async_trait::async_trait;
use serde::Deserialize;

use crate::basket::{OrderNormalizer, PurchasedBasket, PurchasedItem};
use crate::comparer::ProductSelector;

/// One cleaned line as the model returns it.
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
    base_url: String,
    api_key: String,
    model: String,
    reasoning_effort: Option<String>,
}

impl DeepSeekNormalizer {
    /// Talk to DeepSeek's public API with the given key and model.
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_base_url("https://api.deepseek.com".to_string(), api_key, model)
    }

    /// `base_url` is the DeepSeek host in production or a mock server in tests.
    pub fn with_base_url(base_url: String, api_key: String, model: String) -> Self {
        let reasoning_effort = std::env::var("DEEPSEEK_REASONING_EFFORT").ok();
        Self { base_url, api_key, model, reasoning_effort }
    }
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

        let content =
            deepseek_chat(&self.base_url, &self.api_key, &self.model, &prompt, &input, self.reasoning_effort.as_deref())
                .await?;
        let array = extract_json_array(&content)
            .ok_or_else(|| anyhow::anyhow!("no product list in DeepSeek reply: {content}"))?;
        let clean: Vec<CleanItem> = serde_json::from_str(array)?;
        Ok(carry_sizes_over(clean, basket))
    }
}

/// Bridge to the shared deepseek_client crate. Wraps the synchronous HTTP call
/// in tokio::task::spawn_blocking so it plays well with the async runtime.
async fn deepseek_chat(
    base_url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    reasoning_effort: Option<&str>,
) -> anyhow::Result<String> {
    let base_url = base_url.to_string();
    let api_key = api_key.to_string();
    let model = model.to_string();
    let system = system.to_string();
    let user = user.to_string();
    let reasoning_effort = reasoning_effort.map(|s| s.to_string());

    tokio::task::spawn_blocking(move || {
        deepseek_client::chat(
            &base_url,
            &api_key,
            &model,
            &system,
            &user,
            reasoning_effort.as_deref(),
        )
        .map(|r| r.content)
        .map_err(|e| anyhow::anyhow!("{e}"))
    })
    .await
    .map_err(|e| anyhow::anyhow!("DeepSeek task panicked: {e}"))?
}

/// Picks the store product that best matches the bought item, over DeepSeek.
/// Any failure (HTTP error, unparsable or out-of-range reply) yields `None` so
/// the caller falls back to its price heuristic.
pub struct DeepSeekProductSelector {
    base_url: String,
    api_key: String,
    model: String,
    reasoning_effort: Option<String>,
}

impl DeepSeekProductSelector {
    /// Talk to DeepSeek's public API with the given key and model.
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_base_url("https://api.deepseek.com".to_string(), api_key, model)
    }

    /// `base_url` is the DeepSeek host in production or a mock server in tests.
    pub fn with_base_url(base_url: String, api_key: String, model: String) -> Self {
        let reasoning_effort = std::env::var("DEEPSEEK_REASONING_EFFORT").ok();
        Self { base_url, api_key, model, reasoning_effort }
    }
}

#[async_trait]
impl ProductSelector for DeepSeekProductSelector {
    async fn select(&self, description: &str, candidates: &[String]) -> Option<usize> {
        if candidates.is_empty() {
            return None;
        }
        let list = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{i}: {c}"))
            .collect::<Vec<_>>()
            .join("\n");
        let system = "You match a shopping item to the closest supermarket product. \
             Reply with only the 0-based index of the best matching candidate, or -1 \
             if none is a good match. Output just the number.";
        let user = format!("Item: {description}\nCandidates:\n{list}");

        let content =
            deepseek_chat(&self.base_url, &self.api_key, &self.model, system, &user, self.reasoning_effort.as_deref())
                .await
                .ok()?;
        parse_index(&content).filter(|&i| i < candidates.len())
    }
}

/// The first non-negative integer in `text`; `None` for a negative number or
/// no number at all (the model's "-1 / none" answer or a non-numeric reply).
fn parse_index(text: &str) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c == '-' && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
            return None;
        }
        if c.is_ascii_digit() {
            let digits: String = chars[i..].iter().take_while(|c| c.is_ascii_digit()).collect();
            return digits.parse().ok();
        }
    }
    None
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
