use async_trait::async_trait;

use crate::domain::{
    error::DomainError,
    flight::FlightOffer,
    ports::FlightSearchPort,
    search::SearchCriteria,
};

pub struct FakeFlightSearchAdapter {
    result: Result<Vec<FlightOffer>, DomainError>,
}

impl FakeFlightSearchAdapter {
    pub fn returning(offers: Vec<FlightOffer>) -> Self {
        Self { result: Ok(offers) }
    }

    pub fn empty() -> Self {
        Self { result: Ok(vec![]) }
    }

    pub fn failing() -> Self {
        Self { result: Err(DomainError::ProviderError) }
    }
}

#[async_trait]
impl FlightSearchPort for FakeFlightSearchAdapter {
    async fn search(&self, _criteria: &SearchCriteria) -> Result<Vec<FlightOffer>, DomainError> {
        match &self.result {
            Ok(offers) => Ok(offers.clone()),
            Err(e) => Err(match e {
                DomainError::ProviderError => DomainError::ProviderError,
                _ => DomainError::ProviderError,
            }),
        }
    }
}
