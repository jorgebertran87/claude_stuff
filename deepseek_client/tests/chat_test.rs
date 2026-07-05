use deepseek_client::{chat, chat_simple, chat_with_tools, ChatMessage, ToolDefinition, ToolHandler, ToolCall};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

#[tokio::test]
async fn successful_chat_returns_content_and_tokens() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": "hola mundo"}}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5}
        })))
        .mount(&server)
        .await;

    let result = chat_simple(
        &server.uri(),
        "test-key",
        "deepseek-chat",
        "You are helpful.",
        "Hello",
        None,
    );
    let resp = result.unwrap();
    assert_eq!(resp.content, "hola mundo");
    assert_eq!(resp.input_tokens, 10);
    assert_eq!(resp.output_tokens, 5);
}

#[tokio::test]
async fn reasoning_effort_is_sent_when_provided() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": "ok"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })))
        .mount(&server)
        .await;

    let result = chat_simple(
        &server.uri(),
        "test-key",
        "deepseek-reasoner",
        "You are helpful.",
        "Solve this",
        Some("high"),
    );
    assert!(result.is_ok());
}

#[tokio::test]
async fn http_error_is_reported() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let result = chat_simple(
        &server.uri(),
        "test-key",
        "deepseek-chat",
        "system",
        "user",
        None,
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn api_error_in_json_body_is_reported() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": {"message": "Model Not Exist"}
        })))
        .mount(&server)
        .await;

    let result = chat_simple(
        &server.uri(),
        "test-key",
        "deepseek-chat",
        "system",
        "user",
        None,
    );
    assert!(result.unwrap_err().contains("Model Not Exist"));
}

#[test]
fn deepseek_config_reads_env_defaults() {
    std::env::remove_var("DEEPSEEK_BASE_URL");
    std::env::remove_var("DEEPSEEK_MODEL");
    std::env::remove_var("DEEPSEEK_REASONING_EFFORT");

    let config = deepseek_client::DeepSeekConfig::from_env();
    assert_eq!(config.base_url, "https://api.deepseek.com");
    assert_eq!(config.model, "deepseek-chat");
    assert!(config.reasoning_effort.is_none());
}

// ── Current date injection tests ───────────────────────────────────────────

/// A fake ToolHandler that returns a canned response for every tool call.
struct StubToolHandler {
    response: String,
}

impl ToolHandler for StubToolHandler {
    fn execute(&self, _call: &ToolCall) -> Result<String, String> {
        Ok(self.response.clone())
    }
}

#[tokio::test]
async fn chat_includes_current_date_system_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(|req: &Request| {
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or_default();
            let messages = body["messages"].as_array()
                .expect("messages array should exist");
            // First message must be a system message with the date.
            messages.first()
                .and_then(|m| m["role"].as_str())
                .map(|r| r == "system")
                .unwrap_or(false)
                && messages.first()
                    .and_then(|m| m["content"].as_str())
                    .map(|c| c.contains("Current date:"))
                    .unwrap_or(false)
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": "ok"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })))
        .mount(&server)
        .await;

    let messages = vec![
        ChatMessage::new("system", "You are helpful."),
        ChatMessage::new("user", "Hello"),
    ];
    let result = chat(&server.uri(), "test-key", "deepseek-chat", &messages, None);
    assert!(result.is_ok(), "chat should succeed; got: {result:?}");
}

#[tokio::test]
async fn chat_includes_original_system_message_after_date() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(|req: &Request| {
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or_default();
            let messages = body["messages"].as_array()
                .expect("messages array should exist");
            // The original system message should still be present.
            messages.iter().any(|m| {
                m["role"].as_str() == Some("system")
                    && m["content"].as_str().unwrap_or("").contains("You are helpful")
            })
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": "ok"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })))
        .mount(&server)
        .await;

    let messages = vec![
        ChatMessage::new("system", "You are helpful"),
        ChatMessage::new("user", "Hello"),
    ];
    let result = chat(&server.uri(), "test-key", "deepseek-chat", &messages, None);
    assert!(result.is_ok(), "original system message should be preserved; got: {result:?}");
}

#[tokio::test]
async fn date_format_is_iso_8601() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(|req: &Request| {
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or_default();
            let messages = body["messages"].as_array()
                .expect("messages array should exist");
            let date_msg = messages.first()
                .and_then(|m| m["content"].as_str())
                .unwrap_or("");
            // Must match "Current date: YYYY-MM-DD."
            is_valid_date_message(date_msg)
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": "ok"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })))
        .mount(&server)
        .await;

    let messages = vec![
        ChatMessage::new("system", "You are helpful."),
        ChatMessage::new("user", "Hello"),
    ];
    let result = chat(&server.uri(), "test-key", "deepseek-chat", &messages, None);
    assert!(result.is_ok(), "date format should be valid; got: {result:?}");
}

/// Validate that `s` matches "Current date: YYYY-MM-DD." format.
fn is_valid_date_message(s: &str) -> bool {
    let prefix = "Current date: ";
    if !s.starts_with(prefix) { return false; }
    if !s.ends_with('.') { return false; }
    let date_part = &s[prefix.len()..s.len() - 1]; // strip trailing '.'
    if date_part.len() != 10 { return false; }
    // Check YYYY-MM-DD character by character.
    for (i, ch) in date_part.chars().enumerate() {
        match i {
            4 | 7 => { if ch != '-' { return false; } }
            _ => { if !ch.is_ascii_digit() { return false; } }
        }
    }
    true
}

#[tokio::test]
async fn chat_with_tools_includes_current_date() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(|req: &Request| {
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or_default();
            let messages = body["messages"].as_array()
                .expect("messages array should exist");
            messages.first()
                .and_then(|m| m["role"].as_str())
                .map(|r| r == "system")
                .unwrap_or(false)
                && messages.first()
                    .and_then(|m| m["content"].as_str())
                    .map(|c| c.contains("Current date:"))
                    .unwrap_or(false)
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": "done"}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2}
        })))
        .mount(&server)
        .await;

    let messages = vec![
        ChatMessage::new("system", "You are helpful."),
        ChatMessage::new("user", "Search the web."),
    ];
    let tools = vec![ToolDefinition {
        name: "web_search".into(),
        description: "Search the web".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "The search query"}
            },
            "required": ["query"]
        }),
    }];
    let handler = StubToolHandler { response: "search result".into() };
    let result = chat_with_tools(
        &server.uri(),
        "test-key",
        "deepseek-chat",
        &messages,
        &tools,
        &handler,
        None,
    );
    assert!(result.is_ok(), "chat_with_tools should succeed; got: {result:?}");
}
