use regex::Regex;
use strsim::normalized_levenshtein;

const FUZZY_THRESHOLD: f64 = 0.80;

// ── WakeWord ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WakeWord {
    pub value: String,
}

impl WakeWord {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("WakeWord cannot be empty".into());
        }
        Ok(Self { value })
    }

    pub fn matches(&self, text: &str) -> bool {
        let wake = self.value.to_lowercase();
        words_of(text)
            .iter()
            .any(|w| w == &wake || normalized_levenshtein(&wake, w) >= FUZZY_THRESHOLD)
    }

    pub fn extract_order(&self, text: &str) -> Option<String> {
        let wake = self.value.to_lowercase();
        let words = words_of(text);
        for (i, w) in words.iter().enumerate() {
            if w == &wake || normalized_levenshtein(&wake, w) >= FUZZY_THRESHOLD {
                let rest = words[i + 1..].join(" ");
                return if rest.is_empty() { None } else { Some(rest) };
            }
        }
        None
    }
}

fn words_of(text: &str) -> Vec<String> {
    Regex::new(r"\w+")
        .unwrap()
        .find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect()
}

// ── Language ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Language {
    pub code: String,
}

impl Language {
    pub fn new(code: impl Into<String>) -> Result<Self, String> {
        let code = code.into();
        if code.trim().is_empty() {
            return Err("Language code cannot be empty".into());
        }
        Ok(Self { code })
    }

    pub fn lang_prefix(&self) -> &str {
        self.code.split('-').next().unwrap_or(&self.code)
    }
}

// ── AudioCapture ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AudioCapture {
    pub raw:          Vec<u8>,
    pub sample_rate:  u32,
    pub sample_width: u16,
}

impl AudioCapture {
    pub fn new(raw: Vec<u8>, sample_rate: u32, sample_width: u16) -> Self {
        Self { raw, sample_rate, sample_width }
    }
}
