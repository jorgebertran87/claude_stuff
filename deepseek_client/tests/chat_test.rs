use deepseek_client::chat_simple;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
