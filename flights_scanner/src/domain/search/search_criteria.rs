use chrono::{DateTime, Utc};

use crate::domain::{
    airport::IataCode,
    error::DomainError,
    flight::value_objects::{CabinClass, PassengerCount},
};

#[derive(Debug, Clone)]
pub struct SearchCriteria {
    pub origin: IataCode,
    pub destination: IataCode,
    pub departure: DateTime<Utc>,
    pub return_date: Option<DateTime<Utc>>,
    pub passengers: PassengerCount,
    pub cabin: CabinClass,
}

impl SearchCriteria {
    pub fn new(
        origin: IataCode,
        destination: IataCode,
        departure: DateTime<Utc>,
        return_date: Option<DateTime<Utc>>,
        passengers: PassengerCount,
        cabin: CabinClass,
    ) -> Result<Self, DomainError> {
        if origin == destination {
            return Err(DomainError::SameOriginDestination);
        }
        if departure <= Utc::now() {
            return Err(DomainError::DepartureDateInPast);
        }
        Ok(Self { origin, destination, departure, return_date, passengers, cabin })
    }

    pub fn is_round_trip(&self) -> bool {
        self.return_date.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn future() -> DateTime<Utc> {
        Utc::now() + Duration::days(30)
    }

    fn mad() -> IataCode { IataCode::new("MAD").unwrap() }
    fn bcn() -> IataCode { IataCode::new("BCN").unwrap() }
    fn passengers() -> PassengerCount { PassengerCount::new(1, 0, 0).unwrap() }

    #[test]
    fn valid_criteria_created() {
        let c = SearchCriteria::new(mad(), bcn(), future(), None, passengers(), CabinClass::Economy);
        assert!(c.is_ok());
    }

    #[test]
    fn same_origin_destination_rejected() {
        let c = SearchCriteria::new(mad(), mad(), future(), None, passengers(), CabinClass::Economy);
        assert_eq!(c.unwrap_err(), DomainError::SameOriginDestination);
    }

    #[test]
    fn past_departure_date_rejected() {
        let past = Utc::now() - Duration::days(1);
        let c = SearchCriteria::new(mad(), bcn(), past, None, passengers(), CabinClass::Economy);
        assert_eq!(c.unwrap_err(), DomainError::DepartureDateInPast);
    }

    #[test]
    fn round_trip_detected_when_return_date_present() {
        let c = SearchCriteria::new(
            mad(), bcn(), future(), Some(future() + Duration::days(7)), passengers(), CabinClass::Economy,
        ).unwrap();
        assert!(c.is_round_trip());
    }

    #[test]
    fn one_way_detected_when_no_return_date() {
        let c = SearchCriteria::new(mad(), bcn(), future(), None, passengers(), CabinClass::Economy).unwrap();
        assert!(!c.is_round_trip());
    }
}
