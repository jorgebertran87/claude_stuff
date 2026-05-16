use crate::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IataCode(String);

impl IataCode {
    pub fn new(code: &str) -> Result<Self, DomainError> {
        let code = code.trim();
        let valid = code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase());
        if valid {
            Ok(Self(code.to_string()))
        } else {
            Err(DomainError::InvalidIataCode(code.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IataCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_code_is_accepted() {
        let code = IataCode::new("MAD").unwrap();
        assert_eq!(code.as_str(), "MAD");
    }

    #[test]
    fn lowercase_is_rejected() {
        assert_eq!(
            IataCode::new("mad"),
            Err(DomainError::InvalidIataCode("mad".into()))
        );
    }

    #[test]
    fn too_short_is_rejected() {
        assert_eq!(
            IataCode::new("MA"),
            Err(DomainError::InvalidIataCode("MA".into()))
        );
    }

    #[test]
    fn too_long_is_rejected() {
        assert_eq!(
            IataCode::new("MADR"),
            Err(DomainError::InvalidIataCode("MADR".into()))
        );
    }

    #[test]
    fn numeric_char_is_rejected() {
        assert_eq!(
            IataCode::new("MA1"),
            Err(DomainError::InvalidIataCode("MA1".into()))
        );
    }
}
