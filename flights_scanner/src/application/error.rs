use crate::domain::error::DomainError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum AppError {
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("no flights found matching the criteria")]
    NoResults,
    #[error("search provider unavailable")]
    ProviderUnavailable,
}
