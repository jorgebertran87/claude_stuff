use deepseek_client::{ToolCall, ToolHandler};
use std::time::Duration;

/// Maximum characters to return from a fetched page.
const MAX_CONTENT_CHARS: usize = 50_000;

/// Default connect timeout.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default read timeout.
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Fetches a URL and returns its content as plain text.
///
/// HTML pages have their tags stripped to extract readable text.
/// Content is truncated to `MAX_CONTENT_CHARS` to avoid blowing up context.
///
/// Uses native-tls (OpenSSL) for TLS to maximize compatibility with
/// government and legacy servers.
pub struct UrlFetcherTool {
    agent: ureq::Agent,
}

impl UrlFetcherTool {
    /// Creates a tool with default timeouts (10 s connect, 30 s read).
    pub fn new() -> Self {
        Self::with_timeouts(DEFAULT_CONNECT_TIMEOUT, DEFAULT_READ_TIMEOUT)
    }

    /// Creates a tool with custom connect and read timeouts.
    pub fn with_timeouts(connect_timeout: Duration, read_timeout: Duration) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(connect_timeout)
            .timeout_read(read_timeout)
            .build();
        Self { agent }
    }
}

impl ToolHandler for UrlFetcherTool {
    fn execute(&self, call: &ToolCall) -> Result<String, String> {
        let url = call
            .arguments
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'url' argument".to_string())?;

        eprintln!("[url_fetch] GET {url}");

        let response = self
            .agent
            .get(url)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .set("Accept", "text/html, text/plain, */*")
            .call()
            .map_err(|e| format!("URL fetch error for '{url}': {e}"))?;

        let status = response.status();
        if status >= 400 {
            return Err(format!(
                "URL fetch error for '{url}': HTTP {status}"
            ));
        }

        let content_type = response
            .header("Content-Type")
            .unwrap_or("")
            .to_string();

        let body = response
            .into_string()
            .map_err(|e| format!("URL fetch read error for '{url}': {e}"))?;

        let text = if content_type.contains("text/html") || looks_like_html(&body) {
            eprintln!("[url_fetch] HTML detected, stripping tags ({len} bytes raw)", len = body.len());
            strip_html(&body)
        } else {
            body
        };

        let truncated = truncate_content(&text);
        eprintln!("[url_fetch] returned {} bytes", truncated.len());
        Ok(truncated)
    }
}

/// Heuristic: does the content look like HTML?
fn looks_like_html(body: &str) -> bool {
    // Take up to 200 chars, but safely — don't split a multi-byte codepoint.
    let end = body
        .char_indices()
        .take(200)
        .last()
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(body.len());
    let safe_slice = &body[..end.min(body.len())];
    let lower = safe_slice.to_lowercase();
    lower.contains("<html") || lower.contains("<!doctype") || lower.contains("<body")
}

/// Strip HTML tags, returning readable text.
///
/// A simple tag stripper — removes everything between `<` and `>`,
/// collapses whitespace, and trims.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }

    // Collapse runs of whitespace.
    let collapsed: String = result
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    collapsed
}

/// Truncate content to MAX_CONTENT_CHARS, breaking at a newline if possible.
fn truncate_content(text: &str) -> String {
    if text.chars().count() <= MAX_CONTENT_CHARS {
        return text.to_string();
    }

    // Find a char boundary near MAX_CONTENT_CHARS.
    let mut end = MAX_CONTENT_CHARS;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let prefix = &text[..end];
    // Try to break at the last newline.
    if let Some(last_nl) = prefix.rfind('\n') {
        if last_nl > MAX_CONTENT_CHARS / 2 {
            return format!("{}\n\n[Content truncated...]", &text[..last_nl]);
        }
    }

    format!("{prefix}\n\n[Content truncated...]")
}