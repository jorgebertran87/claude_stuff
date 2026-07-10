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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_exact() {
        let ww = WakeWord::new("claudito").unwrap();
        assert!(ww.matches("claudito"));
    }

    #[test]
    fn matches_fuzzy() {
        let ww = WakeWord::new("claudito").unwrap();
        assert!(ww.matches("clauditto"));
    }

    #[test]
    fn matches_rejects_unrelated() {
        let ww = WakeWord::new("claudito").unwrap();
        assert!(!ww.matches("hola mundo"));
    }

    #[test]
    fn extract_order_exact_with_trailing() {
        let ww = WakeWord::new("claudito").unwrap();
        assert_eq!(ww.extract_order("claudito pon música"), Some("pon música".into()));
    }

    #[test]
    fn extract_order_fuzzy_with_trailing() {
        let ww = WakeWord::new("claudito").unwrap();
        assert_eq!(ww.extract_order("clauditto pon música"), Some("pon música".into()));
    }

    #[test]
    fn extract_order_no_trailing() {
        let ww = WakeWord::new("claudito").unwrap();
        assert_eq!(ww.extract_order("claudito"), None);
    }

    #[test]
    fn extract_order_no_match() {
        let ww = WakeWord::new("claudito").unwrap();
        assert_eq!(ww.extract_order("hola mundo"), None);
    }
}

fn words_of(text: &str) -> Vec<String> {
    Regex::new(r"\w+")
        .unwrap()
        .find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect()
}
