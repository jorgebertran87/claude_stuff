use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};

use crate::domain::{
    airport::IataCode,
    error::DomainError,
    flight::{
        entity::Flight,
        value_objects::{CabinClass, FlightNumber, Price},
        FlightOffer,
    },
    ports::FlightSearchPort,
    search::SearchCriteria,
};

pub struct InMemoryFlightSearchAdapter {
    offers: Vec<FlightOffer>,
}

impl Default for InMemoryFlightSearchAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryFlightSearchAdapter {
    pub fn new() -> Self {
        Self { offers: preset_offers() }
    }
}

#[async_trait]
impl FlightSearchPort for InMemoryFlightSearchAdapter {
    async fn search(&self, criteria: &SearchCriteria) -> Result<Vec<FlightOffer>, DomainError> {
        let matching = self
            .offers
            .iter()
            .filter(|o| {
                o.outbound.origin == criteria.origin
                    && o.outbound.destination == criteria.destination
            })
            .cloned()
            .collect();
        Ok(matching)
    }
}

fn dep(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
}

fn preset_offers() -> Vec<FlightOffer> {
    vec![
        FlightOffer::new(
            Flight::new(
                FlightNumber::new("IB3456").unwrap(),
                IataCode::new("MAD").unwrap(),
                IataCode::new("LHR").unwrap(),
                dep(2026, 12, 1, 10),
                dep(2026, 12, 1, 10) + Duration::minutes(150),
                CabinClass::Economy,
            ),
            None,
            Price::new(189.99, "EUR").unwrap(),
            8,
        ),
        FlightOffer::new(
            Flight::new(
                FlightNumber::new("VY7654").unwrap(),
                IataCode::new("MAD").unwrap(),
                IataCode::new("LHR").unwrap(),
                dep(2026, 12, 1, 14),
                dep(2026, 12, 1, 14) + Duration::minutes(130),
                CabinClass::Economy,
            ),
            None,
            Price::new(99.99, "EUR").unwrap(),
            3,
        ),
        FlightOffer::new(
            Flight::new(
                FlightNumber::new("IB3457").unwrap(),
                IataCode::new("LHR").unwrap(),
                IataCode::new("MAD").unwrap(),
                dep(2026, 12, 8, 11),
                dep(2026, 12, 8, 11) + Duration::minutes(155),
                CabinClass::Economy,
            ),
            None,
            Price::new(179.99, "EUR").unwrap(),
            5,
        ),
        FlightOffer::new(
            Flight::new(
                FlightNumber::new("VY1234").unwrap(),
                IataCode::new("BCN").unwrap(),
                IataCode::new("CDG").unwrap(),
                dep(2026, 12, 2, 7),
                dep(2026, 12, 2, 7) + Duration::minutes(90),
                CabinClass::Economy,
            ),
            None,
            Price::new(79.99, "EUR").unwrap(),
            12,
        ),
        FlightOffer::new(
            Flight::new(
                FlightNumber::new("IB6250").unwrap(),
                IataCode::new("MAD").unwrap(),
                IataCode::new("JFK").unwrap(),
                dep(2026, 12, 5, 13),
                dep(2026, 12, 5, 13) + Duration::minutes(540),
                CabinClass::Business,
            ),
            None,
            Price::new(1850.00, "EUR").unwrap(),
            2,
        ),
    ]
}
