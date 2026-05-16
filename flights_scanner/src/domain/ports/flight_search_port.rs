use async_trait::async_trait;

use crate::domain::{
    error::DomainError,
    flight::FlightOffer,
    search::SearchCriteria,
};

#[async_trait]
pub trait FlightSearchPort: Send + Sync {
    async fn search(&self, criteria: &SearchCriteria) -> Result<Vec<FlightOffer>, DomainError>;
}
