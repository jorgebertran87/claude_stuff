use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::flight::{entity::Flight, value_objects::CabinClass, FlightOffer};

#[derive(Serialize)]
pub struct FlightOfferResponse {
    pub id: String,
    pub outbound: FlightResponse,
    pub inbound: Option<FlightResponse>,
    pub price: PriceResponse,
    pub seats_available: u8,
    pub is_round_trip: bool,
    pub total_duration_minutes: u32,
}

#[derive(Serialize)]
pub struct FlightResponse {
    pub number: String,
    pub origin: String,
    pub destination: String,
    pub departure: DateTime<Utc>,
    pub arrival: DateTime<Utc>,
    pub cabin_class: String,
}

#[derive(Serialize)]
pub struct PriceResponse {
    pub amount: f64,
    pub currency: String,
}

impl From<FlightOffer> for FlightOfferResponse {
    fn from(offer: FlightOffer) -> Self {
        Self {
            id: offer.id.to_string(),
            is_round_trip: offer.is_round_trip(),
            total_duration_minutes: offer.total_duration().minutes(),
            inbound: offer.inbound.map(FlightResponse::from),
            price: PriceResponse {
                amount: offer.price.amount(),
                currency: offer.price.currency().to_string(),
            },
            seats_available: offer.seats_available,
            outbound: FlightResponse::from(offer.outbound),
        }
    }
}

impl From<Flight> for FlightResponse {
    fn from(f: Flight) -> Self {
        Self {
            number: f.number.as_str().to_string(),
            origin: f.origin.as_str().to_string(),
            destination: f.destination.as_str().to_string(),
            departure: f.departure,
            arrival: f.arrival,
            cabin_class: cabin_class_label(f.cabin),
        }
    }
}

fn cabin_class_label(cabin: CabinClass) -> String {
    match cabin {
        CabinClass::Economy => "Economy",
        CabinClass::Business => "Business",
        CabinClass::First => "First",
    }
    .to_string()
}
