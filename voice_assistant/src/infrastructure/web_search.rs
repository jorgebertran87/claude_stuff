use deepseek_client::{ToolCall, ToolHandler};

/// Maximum number of search results to return.
const MAX_RESULTS: usize = 5;

/// Maximum characters per snippet to keep results concise.
const MAX_SNIPPET_CHARS: usize = 300;

/// Searches the web via DuckDuckGo Lite (no API key required).
///
/// Scrapes `https://lite.duckduckgo.com/lite/` which returns clean,
/// no-JS HTML.  Parses the result tables to extract title, snippet,
/// and URL for each hit.
pub struct DuckDuckGoSearchTool {
    base_url: String,
}

impl DuckDuckGoSearchTool {
    /// Create a tool pointed at the real DuckDuckGo Lite endpoint.
    pub fn new() -> Self {
        Self {
            base_url: "https://lite.duckduckgo.com".to_string(),
        }
    }

    /// Point the tool at a custom URL (used in tests with wiremock).
    pub fn with_base_url(base_url: String) -> Self {
        Self { base_url }
    }
}

impl ToolHandler for DuckDuckGoSearchTool {
    fn execute(&self, call: &ToolCall) -> Result<String, String> {
        let query = call
            .arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'query' argument".to_string())?;

        let encoded = url_encode(query);
        let url = format!("{}/lite/?q={}", self.base_url, encoded);

        eprintln!("[web_search] query=\"{query}\"");

        let response = ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .set("Accept", "text/html")
            .call()
            .map_err(|e| format!("Search HTTP error: {e}"))?;

        let body = response
            .into_string()
            .map_err(|e| format!("Search read error: {e}"))?;

        let results = parse_ddg_lite_html(&body);

        eprintln!("[web_search] {} results found", results.len());

        if results.is_empty() {
            return Ok("No search results found.".to_string());
        }

        Ok(format_results(&results))
    }
}

/// A parsed search result from DuckDuckGo Lite.
struct SearchResult {
    title: String,
    snippet: String,
    url: String,
}

/// Parse DuckDuckGo Lite HTML into search results.
///
/// DuckDuckGo Lite renders each result as a `<table>` with two rows:
/// 1. `<a href="URL">Title</a>`
/// 2. `<td class="result-snippet">Snippet</td>`
fn parse_ddg_lite_html(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut remaining = html;

    while results.len() < MAX_RESULTS {
        // Find the next result table.
        let table_start = match remaining.find("<table") {
            Some(pos) => pos,
            None => break,
        };
        let table_end = match remaining[table_start..].find("</table>") {
            Some(pos) => table_start + pos + "</table>".len(),
            None => break,
        };
        let table = &remaining[table_start..table_end];

        // Extract the link: <a href="URL" ...>Title</a>
        let title = extract_tag_content(table, "a").unwrap_or_default();
        let url = extract_attribute(table, "a", "href").unwrap_or_default();

        // Extract the snippet: <td class="result-snippet">...</td>
        let snippet = extract_snippet(table);

        if !title.is_empty() && !url.is_empty() {
            let snippet = truncate_snippet(&snippet);
            results.push(SearchResult { title, snippet, url });
        }

        remaining = &remaining[table_end..];
    }

    results
}

/// Extract the snippet from a result-snippet td.
fn extract_snippet(html: &str) -> String {
    // Look for <td class="result-snippet"> or just <td class="result-snippet" ...>
    if let Some(start) = html.find("result-snippet") {
        // Find the closing > of the td tag
        if let Some(content_start) = html[start..].find('>') {
            let content_start = start + content_start + 1;
            if let Some(end) = html[content_start..].find("</td>") {
                return html[content_start..content_start + end].trim().to_string();
            }
        }
    }
    String::new()
}

/// Extract the text content of the first tag with the given name.
fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let start = html.find(&open)?;
    let content_start = html[start..].find('>')? + 1;
    let close = format!("</{}>", tag);
    let content_end = html[start + content_start..].find(&close)?;
    Some(html[start + content_start..start + content_start + content_end]
        .trim()
        .to_string())
}

/// Extract the value of an attribute from the first tag with the given name.
fn extract_attribute(html: &str, tag: &str, attr: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let start = html.find(&open)?;
    let tag_end = html[start..].find('>')?;
    let tag_content = &html[start..start + tag_end];

    let attr_pattern = format!("{}=\"", attr);
    let attr_start = tag_content.find(&attr_pattern)?;
    let value_start = attr_start + attr_pattern.len();
    let value_end = tag_content[value_start..].find('"')?;
    Some(tag_content[value_start..value_start + value_end].to_string())
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
    fn parse_typical_ddg_lite_page() {
        let html = r#"<table><tr><td><a href="https://example.com" rel="nofollow">Example Title</a></td></tr><tr><td class="result-snippet">This is a snippet about the example.</td></tr></table>"#;
        let results = parse_ddg_lite_html(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example Title");
        assert_eq!(results[0].url, "https://example.com");
        assert_eq!(results[0].snippet, "This is a snippet about the example.");
    }

    #[test]
    fn parse_extracts_up_to_max_results() {
        let one_table = r#"<table><tr><td><a href="http://a.com">A</a></td></tr><tr><td class="result-snippet">snippet</td></tr></table>"#;
        let html = one_table.repeat(7);
        let results = parse_ddg_lite_html(&html);
        assert_eq!(results.len(), MAX_RESULTS);
    }

    #[test]
    fn parse_skips_tables_without_links() {
        let html = r#"<table><tr><td>No link here</td></tr></table><table><tr><td><a href="http://x.com">X</a></td></tr><tr><td class="result-snippet">snippet</td></tr></table>"#;
        let results = parse_ddg_lite_html(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "X");
    }

    #[test]
    fn parse_empty_html_returns_no_results() {
        let results = parse_ddg_lite_html("<html></html>");
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
