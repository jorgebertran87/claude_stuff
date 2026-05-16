pub mod dto;
pub mod mapper;

use async_trait::async_trait;
use chrono::Datelike;

use crate::domain::{
    error::DomainError,
    flight::{value_objects::CabinClass, FlightOffer},
    ports::FlightSearchPort,
    search::SearchCriteria,
};

use dto::{AirportSearchResponse, FlightSearchResponse};
use mapper::map_itineraries;

const RAPIDAPI_HOST: &str = "sky-scrapper.p.rapidapi.com";

pub struct SkyScrapperAdapter {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl SkyScrapperAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("https://{RAPIDAPI_HOST}"),
            api_key,
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { client: reqwest::Client::new(), base_url, api_key }
    }

    async fn resolve_airport(&self, iata: &str) -> Result<(String, String), DomainError> {
        let resp: AirportSearchResponse = self
            .client
            .get(format!("{}/api/v1/flights/searchAirport", self.base_url))
            .header("X-RapidAPI-Key", &self.api_key)
            .header("X-RapidAPI-Host", RAPIDAPI_HOST)
            .query(&[("query", iata), ("locale", "en-US")])
            .send()
            .await
            .map_err(|_| DomainError::ProviderError)?
            .json()
            .await
            .map_err(|_| DomainError::ProviderError)?;

        resp.data
            .into_iter()
            .find(|r| r.sky_id.eq_ignore_ascii_case(iata))
            .map(|r| (r.sky_id, r.entity_id))
            .ok_or(DomainError::ProviderError)
    }
}

#[async_trait]
impl FlightSearchPort for SkyScrapperAdapter {
    async fn search(&self, criteria: &SearchCriteria) -> Result<Vec<FlightOffer>, DomainError> {
        let (origin_sky_id, origin_entity_id) =
            self.resolve_airport(criteria.origin.as_str()).await?;
        let (dest_sky_id, dest_entity_id) =
            self.resolve_airport(criteria.destination.as_str()).await?;

        let date = format!(
            "{}-{:02}-{:02}",
            criteria.departure.year(),
            criteria.departure.month(),
            criteria.departure.day()
        );
        let adults = criteria.passengers.adults.to_string();
        let children = criteria.passengers.children.to_string();
        let infants = criteria.passengers.infants.to_string();

        let mut params: Vec<(&str, &str)> = vec![
            ("originSkyId", &origin_sky_id),
            ("destinationSkyId", &dest_sky_id),
            ("originEntityId", &origin_entity_id),
            ("destinationEntityId", &dest_entity_id),
            ("date", &date),
            ("adults", &adults),
            ("children", &children),
            ("infants", &infants),
            ("cabinClass", cabin_class_str(criteria.cabin)),
            ("currency", "EUR"),
            ("countryCode", "UK"),
            ("market", "en-GB"),
            ("locale", "en-GB"),
        ];

        let return_date = criteria
            .return_date
            .map(|ret| format!("{}-{:02}-{:02}", ret.year(), ret.month(), ret.day()));
        if let Some(ref rd) = return_date {
            params.push(("returnDate", rd.as_str()));
        }

        let resp: FlightSearchResponse = self
            .client
            .get(format!("{}/api/v2/flights/searchFlightsWebComplete", self.base_url))
            .header("X-RapidAPI-Key", &self.api_key)
            .header("X-RapidAPI-Host", RAPIDAPI_HOST)
            .query(&params)
            .send()
            .await
            .map_err(|_| DomainError::ProviderError)?
            .json()
            .await
            .map_err(|_| DomainError::ProviderError)?;

        if !resp.status {
            return Err(DomainError::ProviderError);
        }

        let itineraries = resp.data.map(|d| d.itineraries).unwrap_or_default();
        Ok(map_itineraries(&itineraries, criteria.cabin, "EUR"))
    }
}

fn cabin_class_str(cabin: CabinClass) -> &'static str {
    match cabin {
        CabinClass::Economy => "economy",
        CabinClass::Business => "business",
        CabinClass::First => "first",
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use serde_json::json;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::domain::{airport::IataCode, flight::value_objects::PassengerCount, search::SearchCriteria};

    fn make_criteria() -> SearchCriteria {
        SearchCriteria::new(
            IataCode::new("MAD").unwrap(),
            IataCode::new("LHR").unwrap(),
            Utc::now() + Duration::days(30),
            None,
            PassengerCount::new(1, 0, 0).unwrap(),
            CabinClass::Economy,
        )
        .unwrap()
    }

    fn airports_body() -> serde_json::Value {
        json!({
            "status": true,
            "data": [
                { "skyId": "MAD", "entityId": "entity_mad" },
                { "skyId": "LHR", "entityId": "entity_lhr" }
            ]
        })
    }

    fn flights_body() -> serde_json::Value {
        json!({
            "status": true,
            "data": {
                "itineraries": [{
                    "price": { "raw": 189.99 },
                    "legs": [{
                        "origin": { "displayCode": "MAD" },
                        "destination": { "displayCode": "LHR" },
                        "departure": "2026-12-01T10:00:00",
                        "arrival": "2026-12-01T12:30:00",
                        "durationInMinutes": 150,
                        "stopCount": 0,
                        "carriers": { "marketing": [{ "alternateId": "IB" }] },
                        "segments": [{
                            "flightNumber": "3456",
                            "marketingCarrier": { "alternateId": "IB" }
                        }]
                    }]
                }]
            }
        })
    }

    async fn mount_airports(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/api/v1/flights/searchAirport"))
            .respond_with(ResponseTemplate::new(200).set_body_json(airports_body()))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn search_returns_offers() {
        let server = MockServer::start().await;
        mount_airports(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v2/flights/searchFlightsWebComplete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(flights_body()))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        let offers = adapter.search(&make_criteria()).await.unwrap();

        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].outbound.origin.as_str(), "MAD");
        assert_eq!(offers[0].outbound.destination.as_str(), "LHR");
        assert!((offers[0].price.amount() - 189.99).abs() < 0.01);
    }

    #[tokio::test]
    async fn search_returns_empty_when_no_itineraries() {
        let server = MockServer::start().await;
        mount_airports(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v2/flights/searchFlightsWebComplete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": true,
                "data": { "itineraries": [] }
            })))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        let offers = adapter.search(&make_criteria()).await.unwrap();
        assert!(offers.is_empty());
    }

    #[tokio::test]
    async fn search_returns_error_when_airport_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/flights/searchAirport"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": true,
                "data": []
            })))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        let result = adapter.search(&make_criteria()).await;
        assert!(matches!(result, Err(DomainError::ProviderError)));
    }

    #[tokio::test]
    async fn search_returns_error_on_http_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/flights/searchAirport"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        let result = adapter.search(&make_criteria()).await;
        assert!(matches!(result, Err(DomainError::ProviderError)));
    }

    #[tokio::test]
    async fn search_returns_error_when_flights_status_is_failed() {
        let server = MockServer::start().await;
        mount_airports(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v2/flights/searchFlightsWebComplete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": false,
                "data": null
            })))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        let result = adapter.search(&make_criteria()).await;
        assert!(matches!(result, Err(DomainError::ProviderError)));
    }
}
