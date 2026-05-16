use std::collections::HashMap;

use chrono::{TimeZone, Utc};

use crate::domain::{
    airport::IataCode,
    flight::{
        entity::Flight,
        value_objects::{CabinClass, FlightNumber, Price},
        FlightOffer,
    },
};

use super::dto::{Carrier, Itinerary, Leg, Place, Price as SkyPrice, Results, Segment, SkyscannerDateTime};

pub fn map_results(results: &Results, cabin: CabinClass, currency: &str) -> Vec<FlightOffer> {
    results
        .itineraries
        .values()
        .filter_map(|itin| {
            map_itinerary(
                itin,
                &results.legs,
                &results.segments,
                &results.carriers,
                &results.places,
                cabin,
                currency,
            )
        })
        .collect()
}

fn map_itinerary(
    itin: &Itinerary,
    legs: &HashMap<String, Leg>,
    segments: &HashMap<String, Segment>,
    carriers: &HashMap<String, Carrier>,
    places: &HashMap<String, Place>,
    cabin: CabinClass,
    currency: &str,
) -> Option<FlightOffer> {
    let price = itin
        .pricing_options
        .first()
        .and_then(|opt| parse_price(&opt.price, currency))?;

    let outbound = legs
        .get(itin.leg_ids.first()?)
        .and_then(|leg| map_leg(leg, segments, carriers, places, cabin))?;

    let inbound = itin
        .leg_ids
        .get(1)
        .and_then(|id| legs.get(id))
        .and_then(|leg| map_leg(leg, segments, carriers, places, cabin));

    Some(FlightOffer::new(outbound, inbound, price, 9))
}

fn map_leg(
    leg: &Leg,
    segments: &HashMap<String, Segment>,
    carriers: &HashMap<String, Carrier>,
    places: &HashMap<String, Place>,
    cabin: CabinClass,
) -> Option<Flight> {
    let origin = places.get(&leg.origin_place_id).and_then(|p| IataCode::new(&p.iata).ok())?;
    let destination =
        places.get(&leg.destination_place_id).and_then(|p| IataCode::new(&p.iata).ok())?;

    let number = flight_number(leg, segments, carriers)?;
    let departure = to_datetime(&leg.departure_date_time)?;
    let arrival = to_datetime(&leg.arrival_date_time)?;

    Some(Flight::new(number, origin, destination, departure, arrival, cabin))
}

fn flight_number(
    leg: &Leg,
    segments: &HashMap<String, Segment>,
    carriers: &HashMap<String, Carrier>,
) -> Option<FlightNumber> {
    // prefer segment-level data (most precise)
    if let Some(seg) = leg.segment_ids.first().and_then(|id| segments.get(id)) {
        let carrier_iata = carriers.get(&seg.marketing_carrier_id).map(|c| c.iata.as_str())?;
        if let Ok(fn_) = FlightNumber::new(&format!("{}{}", carrier_iata, seg.marketing_flight_number)) {
            return Some(fn_);
        }
    }
    // fallback: leg-level carrier + "0"
    let carrier_iata = leg
        .marketing_carrier_ids
        .first()
        .and_then(|id| carriers.get(id))
        .map(|c| c.iata.as_str())?;
    FlightNumber::new(&format!("{carrier_iata}0")).ok()
}

pub fn parse_price(price: &SkyPrice, currency: &str) -> Option<Price> {
    let raw: f64 = price.amount.parse().ok()?;
    let amount = if price.unit == "PRICE_UNIT_MILLI" { raw / 1000.0 } else { raw };
    Price::new(amount, currency).ok()
}

fn to_datetime(dt: &SkyscannerDateTime) -> Option<chrono::DateTime<Utc>> {
    Utc.with_ymd_and_hms(dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second)
        .single()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::adapters::skyscanner::dto::{
        Carrier, Itinerary, Leg, Place, Price as SkyPrice, PricingOption, Results,
        Segment, SkyscannerDateTime,
    };

    fn dt(h: u32) -> SkyscannerDateTime {
        SkyscannerDateTime { year: 2026, month: 12, day: 1, hour: h, minute: 0, second: 0 }
    }

    fn make_results(one_way: bool) -> Results {
        let mut itineraries = HashMap::new();
        let mut leg_ids = vec!["leg1".to_string()];
        if !one_way {
            leg_ids.push("leg2".to_string());
        }
        itineraries.insert(
            "itin1".to_string(),
            Itinerary {
                pricing_options: vec![PricingOption {
                    price: SkyPrice {
                        amount: "189990".to_string(),
                        unit: "PRICE_UNIT_MILLI".to_string(),
                    },
                }],
                leg_ids,
            },
        );

        let mut legs = HashMap::new();
        legs.insert(
            "leg1".to_string(),
            Leg {
                origin_place_id: "place_mad".to_string(),
                destination_place_id: "place_lhr".to_string(),
                departure_date_time: dt(10),
                arrival_date_time: dt(12),
                duration_in_minutes: 150,
                stop_count: 0,
                marketing_carrier_ids: vec!["carrier_ib".to_string()],
                segment_ids: vec!["seg1".to_string()],
            },
        );
        if !one_way {
            legs.insert(
                "leg2".to_string(),
                Leg {
                    origin_place_id: "place_lhr".to_string(),
                    destination_place_id: "place_mad".to_string(),
                    departure_date_time: dt(15),
                    arrival_date_time: dt(17),
                    duration_in_minutes: 140,
                    stop_count: 0,
                    marketing_carrier_ids: vec!["carrier_ib".to_string()],
                    segment_ids: vec!["seg2".to_string()],
                },
            );
        }

        let mut segments = HashMap::new();
        segments.insert(
            "seg1".to_string(),
            Segment {
                marketing_flight_number: "3456".to_string(),
                marketing_carrier_id: "carrier_ib".to_string(),
            },
        );
        if !one_way {
            segments.insert(
                "seg2".to_string(),
                Segment {
                    marketing_flight_number: "3457".to_string(),
                    marketing_carrier_id: "carrier_ib".to_string(),
                },
            );
        }

        let mut carriers = HashMap::new();
        carriers.insert("carrier_ib".to_string(), Carrier { iata: "IB".to_string() });

        let mut places = HashMap::new();
        places.insert("place_mad".to_string(), Place { iata: "MAD".to_string() });
        places.insert("place_lhr".to_string(), Place { iata: "LHR".to_string() });

        Results { itineraries, legs, segments, carriers, places }
    }

    #[test]
    fn milli_price_is_converted_correctly() {
        let price = SkyPrice { amount: "189990".to_string(), unit: "PRICE_UNIT_MILLI".to_string() };
        let result = parse_price(&price, "EUR").unwrap();
        assert!((result.amount() - 189.99).abs() < 0.001);
        assert_eq!(result.currency(), "EUR");
    }

    #[test]
    fn whole_price_is_used_as_is() {
        let price = SkyPrice { amount: "199".to_string(), unit: "PRICE_UNIT_WHOLE".to_string() };
        let result = parse_price(&price, "EUR").unwrap();
        assert!((result.amount() - 199.0).abs() < 0.001);
    }

    #[test]
    fn invalid_amount_returns_none() {
        let price = SkyPrice { amount: "not-a-number".to_string(), unit: "PRICE_UNIT_MILLI".to_string() };
        assert!(parse_price(&price, "EUR").is_none());
    }

    #[test]
    fn one_way_itinerary_is_mapped_correctly() {
        let results = make_results(true);
        let offers = map_results(&results, CabinClass::Economy, "EUR");
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
        let results = make_results(false);
        let offers = map_results(&results, CabinClass::Economy, "EUR");
        assert_eq!(offers.len(), 1);
        assert!(offers[0].is_round_trip());
        let inbound = offers[0].inbound.as_ref().unwrap();
        assert_eq!(inbound.origin.as_str(), "LHR");
        assert_eq!(inbound.destination.as_str(), "MAD");
    }

    #[test]
    fn offer_with_invalid_iata_in_place_is_skipped() {
        let mut results = make_results(true);
        results.places.insert("place_mad".to_string(), Place { iata: "!!INVALID".to_string() });
        let offers = map_results(&results, CabinClass::Economy, "EUR");
        assert!(offers.is_empty());
    }

    #[test]
    fn offer_with_missing_pricing_is_skipped() {
        let mut results = make_results(true);
        results.itineraries.get_mut("itin1").unwrap().pricing_options.clear();
        let offers = map_results(&results, CabinClass::Economy, "EUR");
        assert!(offers.is_empty());
    }
}
