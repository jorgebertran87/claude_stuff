#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Theme(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeError {
    Required,
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeError::Required => write!(f, "theme is required"),
        }
    }
}

impl Theme {
    pub fn new(value: &str) -> Result<Self, ThemeError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ThemeError::Required);
        }
        Ok(Self(value.to_string()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
