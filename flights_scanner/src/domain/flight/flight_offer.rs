use uuid::Uuid;

use crate::domain::flight::{entity::Flight, value_objects::{Duration, Price}};

#[derive(Debug, Clone)]
pub struct FlightOffer {
    pub id: Uuid,
    pub outbound: Flight,
    pub inbound: Option<Flight>,
    pub price: Price,
    pub seats_available: u8,
}

impl FlightOffer {
    pub fn new(
        outbound: Flight,
        inbound: Option<Flight>,
        price: Price,
        seats_available: u8,
    ) -> Self {
        Self { id: Uuid::new_v4(), outbound, inbound, price, seats_available }
    }

    pub fn is_round_trip(&self) -> bool {
        self.inbound.is_some()
    }

    /// Number of stops (0 = direct). Currently each Flight represents one leg (no intermediate stops).
    pub fn stops(&self) -> u8 {
        0
    }

    pub fn total_duration(&self) -> Duration {
        let outbound = self.outbound.duration();
        match &self.inbound {
            Some(inbound) => outbound + inbound.duration(),
            None => outbound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as CDuration, Utc};
    use crate::domain::{
        airport::IataCode,
        flight::value_objects::{CabinClass, FlightNumber},
    };

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

    fn make_price() -> Price {
        Price::new(199.0, "EUR").unwrap()
    }

    #[test]
    fn is_one_way_when_no_inbound() {
        let offer = FlightOffer::new(make_flight("MAD", "BCN", 90), None, make_price(), 5);
        assert!(!offer.is_round_trip());
    }

    #[test]
    fn is_round_trip_when_inbound_present() {
        let offer = FlightOffer::new(
            make_flight("MAD", "BCN", 90),
            Some(make_flight("BCN", "MAD", 85)),
            make_price(),
            5,
        );
        assert!(offer.is_round_trip());
    }

    #[test]
    fn stops_returns_zero_for_direct_flight() {
        let offer = FlightOffer::new(make_flight("MAD", "BCN", 90), None, make_price(), 5);
        assert_eq!(offer.stops(), 0);
    }

    #[test]
    fn total_duration_sums_both_legs_for_round_trip() {
        let offer = FlightOffer::new(
            make_flight("MAD", "BCN", 90),
            Some(make_flight("BCN", "MAD", 85)),
            make_price(),
            5,
        );
        assert_eq!(offer.total_duration().minutes(), 175);
    }

    #[test]
    fn total_duration_returns_outbound_only_for_one_way() {
        let offer = FlightOffer::new(make_flight("MAD", "LHR", 135), None, make_price(), 3);
        assert_eq!(offer.total_duration().minutes(), 135);
    }
}
