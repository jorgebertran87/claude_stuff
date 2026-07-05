use deepseek_client::{ToolCall, ToolHandler};

/// Maximum number of search results to return.
const MAX_RESULTS: usize = 5;

/// Maximum characters per snippet to keep results concise.
const MAX_SNIPPET_CHARS: usize = 300;

/// Searches the web via a self-hosted SearXNG instance (JSON API).
///
/// Queries `GET /search?format=json&q=...` and extracts title, url,
/// and content (snippet) from each result in the JSON response.
pub struct SearXngSearchTool {
    base_url: String,
}

impl SearXngSearchTool {
    /// Create a tool pointed at the local SearXNG service (Docker).
    pub fn new() -> Self {
        Self {
            base_url: std::env::var("SEARXNG_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        }
    }

    /// Point the tool at a custom URL (used in tests with wiremock).
    pub fn with_base_url(base_url: String) -> Self {
        Self { base_url }
    }
}

impl ToolHandler for SearXngSearchTool {
    fn execute(&self, call: &ToolCall) -> Result<String, String> {
        let query = call
            .arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'query' argument".to_string())?;

        let encoded = url_encode(query);
        let url = format!("{}/search?format=json&q={}", self.base_url, encoded);

        eprintln!("[web_search] query=\"{query}\"");

        let response = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .set("Accept", "application/json")
            .call()
            .map_err(|e| format!("Search HTTP error: {e}"))?;

        let body = response
            .into_string()
            .map_err(|e| format!("Search read error: {e}"))?;

        let results = parse_searxng_json(&body);

        eprintln!("[web_search] {} results found", results.len());

        if results.is_empty() {
            return Ok("No search results found.".to_string());
        }

        Ok(format_results(&results))
    }
}

/// A parsed search result from the SearXNG JSON response.
struct SearchResult {
    title:   String,
    snippet: String,
    url:     String,
}

/// Parse SearXNG JSON response into search results.
///
/// Expected format:
/// ```json
/// { "query": "...", "results": [
///     { "title": "...", "url": "...", "content": "..." },
///     ...
/// ] }
/// ```
fn parse_searxng_json(body: &str) -> Vec<SearchResult> {
    let json: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[web_search] JSON parse error: {e}");
            return Vec::new();
        }
    };

    let results_array = match json.get("results").and_then(|r| r.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    results_array
        .iter()
        .take(MAX_RESULTS)
        .filter_map(|r| {
            let title = r.get("title").and_then(|v| v.as_str()).unwrap_or_default();
            let url = r.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            let snippet = r.get("content").and_then(|v| v.as_str()).unwrap_or_default();
            if title.is_empty() || url.is_empty() {
                return None;
            }
            Some(SearchResult {
                title:   title.to_string(),
                snippet: truncate_snippet(snippet),
                url:     url.to_string(),
            })
        })
        .collect()
}

/// Truncate a snippet to MAX_SNIPPET_CHARS, breaking at word boundaries.
fn truncate_snippet(s: &str) -> String {
    if s.chars().count() <= MAX_SNIPPET_CHARS {
        return s.to_string();
    }
    let mut end = MAX_SNIPPET_CHARS;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let truncated = &s[..end];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &s[..last_space])
    } else {
        format!("{truncated}...")
    }
}

/// Format results as plain text.
fn format_results(results: &[SearchResult]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{}. {}\n   {}\n   {}\n",
                i + 1,
                r.title,
                r.snippet,
                r.url
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Minimal URL-encoding for the query parameter.
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push(hex_char(b >> 4));
                result.push(hex_char(b & 0x0F));
            }
        }
    }
    result
}

fn hex_char(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_searxng_json_results() {
        let json = r#"{
            "query": "rust",
            "results": [
                {
                    "title": "Rust Programming Language",
                    "url": "https://www.rust-lang.org/",
                    "content": "A language empowering everyone."
                }
            ]
        }"#;
        let results = parse_searxng_json(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming Language");
        assert_eq!(results[0].url, "https://www.rust-lang.org/");
        assert_eq!(results[0].snippet, "A language empowering everyone.");
    }

    #[test]
    fn parse_searxng_json_respects_max_results() {
        let one = r#"{"title":"T","url":"http://a.com","content":"s"}"#;
        let results_array = (0..7).map(|_| one).collect::<Vec<_>>().join(",");
        let json = format!(r#"{{"query":"q","results":[{results_array}]}}"#);
        let results = parse_searxng_json(&json);
        assert_eq!(results.len(), MAX_RESULTS);
    }

    #[test]
    fn parse_searxng_json_skips_results_without_title() {
        let json = r#"{
            "query": "q",
            "results": [
                {"url": "http://a.com", "content": "s"},
                {"title": "B", "url": "http://b.com", "content": "s2"}
            ]
        }"#;
        let results = parse_searxng_json(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "B");
    }

    #[test]
    fn parse_searxng_empty_results() {
        let json = r#"{"query": "q", "results": []}"#;
        let results = parse_searxng_json(json);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn parse_searxng_invalid_json_returns_empty() {
        let results = parse_searxng_json("not json at all");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn truncate_short_snippet_is_unchanged() {
        let s = "Hello world";
        assert_eq!(truncate_snippet(s), "Hello world");
    }

    #[test]
    fn url_encode_spaces_become_plus() {
        assert_eq!(url_encode("hello world"), "hello+world");
    }

    #[test]
    fn url_encode_special_chars() {
        let encoded = url_encode("rust & go");
        assert!(encoded.contains("%26"));
    }
}
