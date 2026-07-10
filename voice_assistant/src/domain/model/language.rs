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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_prefix_strips_region() {
        let lang = Language::new("es-ES").unwrap();
        assert_eq!(lang.lang_prefix(), "es");
    }

    #[test]
    fn lang_prefix_returns_code_when_no_dash() {
        let lang = Language::new("en").unwrap();
        assert_eq!(lang.lang_prefix(), "en");
    }
}
