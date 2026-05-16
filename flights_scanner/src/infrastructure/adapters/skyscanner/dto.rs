use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Request ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CreateRequest {
    pub query: Query,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub market: String,
    pub locale: String,
    pub currency: String,
    pub query_legs: Vec<QueryLeg>,
    pub adults: u8,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children_ages: Vec<u8>,
    pub cabin_class: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryLeg {
    pub origin_place_id: PlaceId,
    pub destination_place_id: PlaceId,
    pub date: QueryDate,
}

#[derive(Serialize)]
pub struct PlaceId {
    pub iata: String,
}

#[derive(Serialize)]
pub struct QueryDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

// ── Response ──────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub session_token: String,
    pub status: ResponseStatus,
    pub content: Option<Content>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum ResponseStatus {
    #[serde(rename = "RESULT_STATUS_INCOMPLETE")]
    Incomplete,
    #[serde(rename = "RESULT_STATUS_COMPLETE")]
    Complete,
    #[serde(rename = "RESULT_STATUS_FAILED")]
    Failed,
}

#[derive(Deserialize, Debug)]
pub struct Content {
    pub results: Results,
}

#[derive(Deserialize, Debug, Default)]
pub struct Results {
    #[serde(default)]
    pub itineraries: HashMap<String, Itinerary>,
    #[serde(default)]
    pub legs: HashMap<String, Leg>,
    #[serde(default)]
    pub segments: HashMap<String, Segment>,
    #[serde(default)]
    pub carriers: HashMap<String, Carrier>,
    #[serde(default)]
    pub places: HashMap<String, Place>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Itinerary {
    pub pricing_options: Vec<PricingOption>,
    pub leg_ids: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct PricingOption {
    pub price: Price,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Price {
    pub amount: String,
    pub unit: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    pub origin_place_id: String,
    pub destination_place_id: String,
    pub departure_date_time: SkyscannerDateTime,
    pub arrival_date_time: SkyscannerDateTime,
    pub duration_in_minutes: u32,
    pub stop_count: u8,
    pub marketing_carrier_ids: Vec<String>,
    #[serde(default)]
    pub segment_ids: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SkyscannerDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub marketing_flight_number: String,
    pub marketing_carrier_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Carrier {
    pub iata: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Place {
    pub iata: String,
}
