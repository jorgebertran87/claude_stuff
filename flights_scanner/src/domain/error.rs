#[derive(thiserror::Error, Debug, PartialEq)]
pub enum DomainError {
    #[error("invalid IATA code: '{0}'")]
    InvalidIataCode(String),
    #[error("origin and destination must differ")]
    SameOriginDestination,
    #[error("departure date must be in the future")]
    DepartureDateInPast,
    #[error("invalid passenger count: {0}")]
    InvalidPassengerCount(String),
    #[error("price must be positive")]
    InvalidPrice,
    #[error("duration must be greater than zero")]
    InvalidDuration,
    #[error("invalid flight number: '{0}'")]
    InvalidFlightNumber(String),
    #[error("search provider error")]
    ProviderError,
}
