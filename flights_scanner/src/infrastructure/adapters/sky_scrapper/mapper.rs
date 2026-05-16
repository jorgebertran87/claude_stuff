use chrono::Utc;

use crate::domain::{
    airport::IataCode,
    flight::{
        entity::Flight,
        value_objects::{CabinClass, FlightNumber, Price},
        FlightOffer,
    },
};

use super::dto::{Itinerary, Leg};

pub fn map_itineraries(itineraries: &[Itinerary], cabin: CabinClass, currency: &str) -> Vec<FlightOffer> {
    itineraries.iter().filter_map(|itin| map_itinerary(itin, cabin, currency)).collect()
}

fn map_itinerary(itin: &Itinerary, cabin: CabinClass, currency: &str) -> Option<FlightOffer> {
    let price = Price::new(itin.price.raw, currency).ok()?;
    let outbound = map_leg(itin.legs.first()?, cabin)?;
    let inbound = itin.legs.get(1).and_then(|leg| map_leg(leg, cabin));
    Some(FlightOffer::new(outbound, inbound, price, 9))
}

fn map_leg(leg: &Leg, cabin: CabinClass) -> Option<Flight> {
    let origin = IataCode::new(&leg.origin.display_code).ok()?;
    let destination = IataCode::new(&leg.destination.display_code).ok()?;
    let number = flight_number(leg)?;
    let departure = parse_datetime(&leg.departure)?;
    let arrival = parse_datetime(&leg.arrival)?;
    Some(Flight::new(number, origin, destination, departure, arrival, cabin))
}

fn flight_number(leg: &Leg) -> Option<FlightNumber> {
    if let Some(seg) = leg.segments.first() {
        let carrier = &seg.marketing_carrier.alternate_id;
        if let Ok(fn_) = FlightNumber::new(&format!("{}{}", carrier, seg.flight_number)) {
            return Some(fn_);
        }
    }
    let carrier = leg.carriers.marketing.first().map(|c| c.alternate_id.as_str())?;
    FlightNumber::new(&format!("{carrier}0")).ok()
}

fn parse_datetime(s: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|ndt| ndt.and_utc())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapters::sky_scrapper::dto::{
        Carrier, Carriers, Itinerary, Leg, Place, Segment, SkyPrice,
    };

    fn make_leg(origin: &str, dest: &str, dep: &str, arr: &str, carrier: &str, num: &str) -> Leg {
        Leg {
            origin: Place { display_code: origin.to_string() },
            destination: Place { display_code: dest.to_string() },
            departure: dep.to_string(),
            arrival: arr.to_string(),
            duration_in_minutes: 150,
            stop_count: 0,
            carriers: Carriers { marketing: vec![Carrier { alternate_id: carrier.to_string() }] },
            segments: vec![Segment {
                flight_number: num.to_string(),
                marketing_carrier: Carrier { alternate_id: carrier.to_string() },
            }],
        }
    }

    fn one_way(price: f64) -> Itinerary {
        Itinerary {
            price: SkyPrice { raw: price },
            legs: vec![make_leg("MAD", "LHR", "2026-12-01T10:00:00", "2026-12-01T12:30:00", "IB", "3456")],
        }
    }

    #[test]
    fn one_way_itinerary_is_mapped() {
        let offers = map_itineraries(&[one_way(189.99)], CabinClass::Economy, "EUR");
        assert_eq!(offers.len(), 1);
        let offer = &offers[0];
        assert_eq!(offer.outbound.origin.as_str(), "MAD");
        assert_eq!(offer.outbound.destination.as_str(), "LHR");
        assert_eq!(offer.outbound.number.as_str(), "IB3456");
        assert!(!offer.is_round_trip());
        assert!((offer.price.amount() - 189.99).abs() < 0.001);
    }

    #[test]
    fn round_trip_itinerary_has_inbound_leg() {
        let itin = Itinerary {
            price: SkyPrice { raw: 349.99 },
            legs: vec![
                make_leg("MAD", "LHR", "2026-12-01T10:00:00", "2026-12-01T12:30:00", "IB", "3456"),
                make_leg("LHR", "MAD", "2026-12-08T15:00:00", "2026-12-08T17:30:00", "IB", "3457"),
            ],
        };
        let offers = map_itineraries(&[itin], CabinClass::Economy, "EUR");
        assert_eq!(offers.len(), 1);
        assert!(offers[0].is_round_trip());
        let inbound = offers[0].inbound.as_ref().unwrap();
        assert_eq!(inbound.origin.as_str(), "LHR");
        assert_eq!(inbound.destination.as_str(), "MAD");
    }

    #[test]
    fn invalid_display_code_skips_offer() {
        let itin = Itinerary {
            price: SkyPrice { raw: 99.99 },
            legs: vec![make_leg("!!BAD", "LHR", "2026-12-01T10:00:00", "2026-12-01T12:30:00", "IB", "3456")],
        };
        assert!(map_itineraries(&[itin], CabinClass::Economy, "EUR").is_empty());
    }

    #[test]
    fn zero_price_skips_offer() {
        assert!(map_itineraries(&[one_way(0.0)], CabinClass::Economy, "EUR").is_empty());
    }
}
