use serde::Deserialize;

// ── Airport search ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AirportSearchResponse {
    pub status: bool,
    pub data: Vec<AirportResult>,
}

#[derive(Deserialize)]
pub struct AirportResult {
    pub navigation: AirportNavigation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirportNavigation {
    pub relevant_flight_params: RelevantFlightParams,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelevantFlightParams {
    pub sky_id: String,
    pub entity_id: String,
}

// ── Flight search ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FlightSearchResponse {
    pub status: bool,
    pub data: Option<FlightSearchData>,
}

#[derive(Deserialize)]
pub struct FlightSearchData {
    #[serde(default)]
    pub itineraries: Vec<Itinerary>,
}

#[derive(Deserialize)]
pub struct Itinerary {
    pub price: SkyPrice,
    pub legs: Vec<Leg>,
}

#[derive(Deserialize)]
pub struct SkyPrice {
    pub raw: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    pub origin: Place,
    pub destination: Place,
    pub departure: String,
    pub arrival: String,
    pub duration_in_minutes: u32,
    pub stop_count: u8,
    pub carriers: Carriers,
    pub segments: Vec<Segment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub display_code: String,
}

#[derive(Deserialize)]
pub struct Carriers {
    pub marketing: Vec<Carrier>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Carrier {
    pub alternate_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub flight_number: String,
    pub marketing_carrier: Carrier,
}
