use std::fs::OpenOptions;
use std::io::Write;

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub total_cost_usd: f64,
    pub session_id: Option<String>,
    pub result: String,
}

pub fn log_token_usage(order: &str, usage: &TokenUsage, log_file: &str) {
    let total = usage.input_tokens
        + usage.output_tokens
        + usage.cache_read_input_tokens
        + usage.cache_creation_input_tokens;
    let log_line = format!(
        "Claude order: {} | Tokens used — input: {}, output: {}, \
         cache_read: {}, cache_creation: {}, total: {} | cost: ${:.6} USD",
        order,
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_read_input_tokens,
        usage.cache_creation_input_tokens,
        total,
        usage.total_cost_usd,
    );
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file) {
        let _ = writeln!(file, "{}", log_line);
    }
}

pub fn parse_result_json(json: &str) -> Result<TokenUsage, String> {
    let result     = extract_str(json, "\"result\":")         .unwrap_or_default();
    let cost_str   = extract_str(json, "\"total_cost_usd\":") .unwrap_or_default();
    let session_id = extract_str(json, "\"session_id\":");
    let cost: f64  = cost_str.parse().unwrap_or(0.0);

    Ok(TokenUsage {
        input_tokens:                extract_u64(json, "\"input_tokens\":"),
        output_tokens:               extract_u64(json, "\"output_tokens\":"),
        cache_read_input_tokens:     extract_u64(json, "\"cache_read_input_tokens\":"),
        cache_creation_input_tokens: extract_u64(json, "\"cache_creation_input_tokens\":"),
        total_cost_usd: cost,
        session_id,
        result,
    })
}

pub fn extract_u64(json: &str, key: &str) -> u64 {
    json.find(key)
        .and_then(|pos| {
            let rest = json[pos + key.len()..].trim_start();
            rest.split(|c: char| !c.is_ascii_digit()).next()
        })
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

pub fn extract_str(json: &str, key: &str) -> Option<String> {
    let pos = json.find(key)?;
    let rest = json[pos + key.len()..].trim_start();
    if rest.starts_with('"') {
        let inner = &rest[1..];
        let mut result = String::new();
        let mut chars = inner.chars();
        loop {
            match chars.next()? {
                '\\' => match chars.next()? {
                    '"'  => result.push('"'),
                    'n'  => result.push('\n'),
                    't'  => result.push('\t'),
                    '\\' => result.push('\\'),
                    c    => { result.push('\\'); result.push(c); }
                },
                '"' => return Some(result),
                c   => result.push(c),
            }
        }
    } else {
        let end = rest.find(|c: char| c == ',' || c == '}' || c == '\n')?;
        Some(rest[..end].trim().to_string())
    }
}
