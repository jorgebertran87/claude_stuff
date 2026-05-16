use std::sync::Arc;

use crate::{
    application::error::AppError,
    domain::{
        flight::{value_objects::Price, FlightOffer},
        ports::FlightSearchPort,
        search::SearchCriteria,
    },
};

#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub max_price: Option<Price>,
    pub max_stops: Option<u8>,
    pub sort_by: SortBy,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum SortBy {
    #[default]
    Price,
    Duration,
}

pub struct SearchFlightsUseCase<S: FlightSearchPort + ?Sized> {
    port: Arc<S>,
}

impl<S: FlightSearchPort + ?Sized> SearchFlightsUseCase<S> {
    pub fn new(port: Arc<S>) -> Self {
        Self { port }
    }

    pub async fn execute(
        &self,
        criteria: SearchCriteria,
        filters: SearchFilters,
    ) -> Result<Vec<FlightOffer>, AppError> {
        let mut offers = self
            .port
            .search(&criteria)
            .await
            .map_err(|_| AppError::ProviderUnavailable)?;

        if let Some(max_price) = &filters.max_price {
            offers.retain(|o| &o.price <= max_price);
        }

        if let Some(max_stops) = filters.max_stops {
            offers.retain(|o| o.stops() <= max_stops);
        }

        match filters.sort_by {
            SortBy::Price => {
                offers.sort_by(|a, b| {
                    a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortBy::Duration => {
                offers.sort_by_key(|o| o.total_duration().minutes());
            }
        }

        if offers.is_empty() {
            return Err(AppError::NoResults);
        }

        Ok(offers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as CDuration, Utc};

    use crate::{
        application::fakes::FakeFlightSearchAdapter,
        domain::{
            airport::IataCode,
            flight::{
                entity::Flight,
                value_objects::{CabinClass, FlightNumber, PassengerCount, Price},
                FlightOffer,
            },
            search::SearchCriteria,
        },
    };

    fn criteria() -> SearchCriteria {
        SearchCriteria::new(
            IataCode::new("MAD").unwrap(),
            IataCode::new("LHR").unwrap(),
            Utc::now() + CDuration::days(30),
            None,
            PassengerCount::new(1, 0, 0).unwrap(),
            CabinClass::Economy,
        )
        .unwrap()
    }

    fn make_flight(origin: &str, dest: &str, duration_mins: i64) -> Flight {
        let now = Utc::now();
        Flight::new(
            FlightNumber::new("IB1234").unwrap(),
            IataCode::new(origin).unwrap(),
            IataCode::new(dest).unwrap(),
            now,
            now + CDuration::minutes(duration_mins),
            CabinClass::Economy,
        )
    }

    fn offer_with_price(amount: f64) -> FlightOffer {
        FlightOffer::new(make_flight("MAD", "LHR", 120), None, Price::new(amount, "EUR").unwrap(), 5)
    }

    fn offer_with_duration(duration_mins: i64) -> FlightOffer {
        FlightOffer::new(
            make_flight("MAD", "LHR", duration_mins),
            None,
            Price::new(200.0, "EUR").unwrap(),
            5,
        )
    }

    fn offer_round_trip(price: f64) -> FlightOffer {
        FlightOffer::new(
            make_flight("MAD", "LHR", 120),
            Some(make_flight("LHR", "MAD", 110)),
            Price::new(price, "EUR").unwrap(),
            5,
        )
    }

    #[tokio::test]
    async fn returns_offers_for_valid_criteria() {
        let offers = vec![offer_with_price(200.0), offer_with_price(300.0)];
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::returning(offers)));
        let result = svc.execute(criteria(), SearchFilters::default()).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn returns_no_results_error_when_empty() {
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::empty()));
        let err = svc.execute(criteria(), SearchFilters::default()).await.unwrap_err();
        assert_eq!(err, AppError::NoResults);
    }

    #[tokio::test]
    async fn filters_by_max_price() {
        let offers = vec![offer_with_price(100.0), offer_with_price(200.0), offer_with_price(300.0)];
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::returning(offers)));
        let filters = SearchFilters {
            max_price: Some(Price::new(200.0, "EUR").unwrap()),
            ..Default::default()
        };
        let result = svc.execute(criteria(), filters).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|o| o.price.amount() <= 200.0));
    }

    #[tokio::test]
    async fn filters_by_max_stops() {
        let offers = vec![offer_with_price(100.0), offer_with_price(200.0)];
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::returning(offers)));
        let filters = SearchFilters { max_stops: Some(0), ..Default::default() };
        let result = svc.execute(criteria(), filters).await.unwrap();
        assert_eq!(result.len(), 2); // all are direct (0 stops)
    }

    #[tokio::test]
    async fn sorts_by_price_ascending_by_default() {
        let offers = vec![offer_with_price(300.0), offer_with_price(100.0), offer_with_price(200.0)];
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::returning(offers)));
        let result = svc.execute(criteria(), SearchFilters::default()).await.unwrap();
        let prices: Vec<f64> = result.iter().map(|o| o.price.amount()).collect();
        assert_eq!(prices, vec![100.0, 200.0, 300.0]);
    }

    #[tokio::test]
    async fn sorts_by_duration() {
        let offers = vec![offer_with_duration(180), offer_with_duration(90), offer_with_duration(120)];
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::returning(offers)));
        let filters = SearchFilters { sort_by: SortBy::Duration, ..Default::default() };
        let result = svc.execute(criteria(), filters).await.unwrap();
        let durations: Vec<u32> = result.iter().map(|o| o.total_duration().minutes()).collect();
        assert_eq!(durations, vec![90, 120, 180]);
    }

    #[tokio::test]
    async fn propagates_provider_error_as_provider_unavailable() {
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::failing()));
        let err = svc.execute(criteria(), SearchFilters::default()).await.unwrap_err();
        assert_eq!(err, AppError::ProviderUnavailable);
    }

    #[tokio::test]
    async fn round_trip_search_returns_round_trip_offers() {
        let offers = vec![offer_round_trip(450.0)];
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::returning(offers)));
        let result = svc.execute(criteria(), SearchFilters::default()).await.unwrap();
        assert!(result[0].is_round_trip());
    }

    #[tokio::test]
    async fn no_results_after_price_filter_returns_error() {
        let offers = vec![offer_with_price(500.0)];
        let svc = SearchFlightsUseCase::new(Arc::new(FakeFlightSearchAdapter::returning(offers)));
        let filters = SearchFilters {
            max_price: Some(Price::new(100.0, "EUR").unwrap()),
            ..Default::default()
        };
        let err = svc.execute(criteria(), filters).await.unwrap_err();
        assert_eq!(err, AppError::NoResults);
    }
}
