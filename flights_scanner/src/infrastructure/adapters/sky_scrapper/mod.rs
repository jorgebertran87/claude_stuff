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

use dto::AirportSearchResponse;

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
        let bytes = self
            .client
            .get(format!("{}/api/v1/flights/searchAirport", self.base_url))
            .header("X-RapidAPI-Key", &self.api_key)
            .header("X-RapidAPI-Host", RAPIDAPI_HOST)
            .query(&[("query", iata), ("locale", "en-US")])
            .send()
            .await
            .map_err(|e| {
                eprintln!("[sky_scrapper] airport request failed for {iata}: {e}");
                DomainError::ProviderError
            })?
            .bytes()
            .await
            .map_err(|_| DomainError::ProviderError)?;

        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
                eprintln!("[sky_scrapper] API error: {msg}");
                return Err(DomainError::ProviderError);
            }
        }

        let resp: AirportSearchResponse = serde_json::from_slice(&bytes).map_err(|e| {
            eprintln!("[sky_scrapper] airport parse error for {iata}: {e}");
            eprintln!("[sky_scrapper] raw: {}", String::from_utf8_lossy(&bytes));
            DomainError::ProviderError
        })?;

        resp.data
            .into_iter()
            .map(|r| r.navigation.relevant_flight_params)
            .find(|p| p.sky_id.eq_ignore_ascii_case(iata))
            .map(|p| (p.sky_id, p.entity_id))
            .ok_or_else(|| {
                eprintln!("[sky_scrapper] airport {iata} not found in results");
                DomainError::ProviderError
            })
    }
}

#[async_trait]
impl FlightSearchPort for SkyScrapperAdapter {
    async fn search(&self, criteria: &SearchCriteria) -> Result<Vec<FlightOffer>, DomainError> {
        let (origin_sky_id, _) = self.resolve_airport(criteria.origin.as_str()).await?;

        let date = format!(
            "{}-{:02}-{:02}",
            criteria.departure.year(),
            criteria.departure.month(),
            criteria.departure.day()
        );
        let adults = criteria.passengers.adults.to_string();
        let one_way = if criteria.return_date.is_some() { "false" } else { "true" };

        let mut params: Vec<(&str, &str)> = vec![
            ("originSkyId", &origin_sky_id),
            ("oneWay", one_way),
            ("date", &date),
            ("adults", &adults),
            ("cabinClass", cabin_class_str(criteria.cabin)),
            ("currency", "USD"),
        ];

        let return_date = criteria
            .return_date
            .map(|ret| format!("{}-{:02}-{:02}", ret.year(), ret.month(), ret.day()));
        if let Some(ref rd) = return_date {
            params.push(("returnDate", rd.as_str()));
        }

        eprintln!("[sky_scrapper] calling searchFlightEverywhereDetails params: {params:?}");

        let bytes = self
            .client
            .get(format!("{}/api/v2/flights/searchFlightEverywhereDetails", self.base_url))
            .header("X-RapidAPI-Key", &self.api_key)
            .header("X-RapidAPI-Host", RAPIDAPI_HOST)
            .query(&params)
            .send()
            .await
            .map_err(|e| {
                eprintln!("[sky_scrapper] request failed: {e}");
                DomainError::ProviderError
            })?
            .bytes()
            .await
            .map_err(|_| DomainError::ProviderError)?;

        eprintln!("[sky_scrapper] raw: {}", String::from_utf8_lossy(&bytes));

        let resp: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            eprintln!("[sky_scrapper] parse error: {e}");
            DomainError::ProviderError
        })?;

        if !resp.get("status").and_then(|s| s.as_bool()).unwrap_or(false) {
            eprintln!("[sky_scrapper] status=false");
            return Err(DomainError::ProviderError);
        }

        // TODO: implement proper mapping once we see the response structure
        Ok(vec![])
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

    fn airport_body() -> serde_json::Value {
        json!({
            "status": true,
            "data": [{
                "navigation": {
                    "relevantFlightParams": { "skyId": "MAD", "entityId": "entity_mad" }
                }
            }]
        })
    }

    async fn mount_origin_airport(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/api/v1/flights/searchAirport"))
            .respond_with(ResponseTemplate::new(200).set_body_json(airport_body()))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn search_succeeds_and_returns_results() {
        let server = MockServer::start().await;
        mount_origin_airport(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v2/flights/searchFlightEverywhereDetails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": true,
                "data": []
            })))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        // succeeds without error (mapping is implemented once real response is known)
        assert!(adapter.search(&make_criteria()).await.is_ok());
    }

    #[tokio::test]
    async fn search_returns_empty_when_status_true_no_data() {
        let server = MockServer::start().await;
        mount_origin_airport(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v2/flights/searchFlightEverywhereDetails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": true,
                "data": []
            })))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        assert!(adapter.search(&make_criteria()).await.unwrap().is_empty());
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
        assert!(matches!(adapter.search(&make_criteria()).await, Err(DomainError::ProviderError)));
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
        assert!(matches!(adapter.search(&make_criteria()).await, Err(DomainError::ProviderError)));
    }

    #[tokio::test]
    async fn search_returns_error_when_flights_status_is_failed() {
        let server = MockServer::start().await;
        mount_origin_airport(&server).await;
        Mock::given(method("GET"))
            .and(path("/api/v2/flights/searchFlightEverywhereDetails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": false,
                "message": "Something went wrong."
            })))
            .mount(&server)
            .await;

        let adapter = SkyScrapperAdapter::with_base_url("key".into(), server.uri());
        assert!(matches!(adapter.search(&make_criteria()).await, Err(DomainError::ProviderError)));
    }
}
