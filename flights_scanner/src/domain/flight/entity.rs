use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    airport::IataCode,
    flight::value_objects::{CabinClass, Duration, FlightNumber},
};

#[derive(Debug, Clone)]
pub struct Flight {
    pub id: Uuid,
    pub number: FlightNumber,
    pub origin: IataCode,
    pub destination: IataCode,
    pub departure: DateTime<Utc>,
    pub arrival: DateTime<Utc>,
    pub cabin: CabinClass,
}

impl Flight {
    pub fn new(
        number: FlightNumber,
        origin: IataCode,
        destination: IataCode,
        departure: DateTime<Utc>,
        arrival: DateTime<Utc>,
        cabin: CabinClass,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            number,
            origin,
            destination,
            departure,
            arrival,
            cabin,
        }
    }

    pub fn duration(&self) -> Duration {
        let mins = (self.arrival - self.departure).num_minutes().max(1) as u32;
        Duration::from_minutes(mins).expect("arrival is after departure")
    }
}
