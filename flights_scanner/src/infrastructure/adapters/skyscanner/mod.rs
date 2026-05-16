pub mod dto;
pub mod mapper;

use async_trait::async_trait;
use chrono::Datelike;

use crate::domain::{
    error::DomainError,
    flight::FlightOffer,
    ports::FlightSearchPort,
    search::SearchCriteria,
    flight::value_objects::CabinClass,
};

use dto::{CreateRequest, Query, QueryDate, QueryLeg, PlaceId, ResponseStatus, SearchResponse};
use mapper::map_results;

const MAX_POLLS: usize = 10;

pub struct SkyscannerAdapter {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    poll_delay_ms: u64,
}

impl SkyscannerAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://partners.api.skyscanner.net/apiservices/v3/flights/live/search"
                .to_string(),
            api_key,
            poll_delay_ms: 500,
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            poll_delay_ms: 0,
        }
    }

    async fn create_session(&self, criteria: &SearchCriteria) -> Result<SearchResponse, DomainError> {
        let body = build_create_request(criteria);
        self.client
            .post(format!("{}/create", self.base_url))
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|_| DomainError::ProviderError)?
            .json::<SearchResponse>()
            .await
            .map_err(|_| DomainError::ProviderError)
    }

    async fn poll_session(&self, token: &str) -> Result<SearchResponse, DomainError> {
        self.client
            .post(format!("{}/poll/{}", self.base_url, token))
            .header("x-api-key", &self.api_key)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|_| DomainError::ProviderError)?
            .json::<SearchResponse>()
            .await
            .map_err(|_| DomainError::ProviderError)
    }
}

#[async_trait]
impl FlightSearchPort for SkyscannerAdapter {
    async fn search(&self, criteria: &SearchCriteria) -> Result<Vec<FlightOffer>, DomainError> {
        let mut response = self.create_session(criteria).await?;

        if response.status == ResponseStatus::Failed {
            return Err(DomainError::ProviderError);
        }

        for _ in 0..MAX_POLLS {
            if response.status == ResponseStatus::Complete {
                break;
            }
            if self.poll_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.poll_delay_ms)).await;
            }
            response = self.poll_session(&response.session_token.clone()).await?;
            if response.status == ResponseStatus::Failed {
                return Err(DomainError::ProviderError);
            }
        }

        let results = response
            .content
            .map(|c| c.results)
            .unwrap_or_default();

        Ok(map_results(&results, criteria.cabin, "EUR"))
    }
}

fn build_create_request(criteria: &SearchCriteria) -> CreateRequest {
    let mut legs = vec![QueryLeg {
        origin_place_id: PlaceId { iata: criteria.origin.as_str().to_string() },
        destination_place_id: PlaceId { iata: criteria.destination.as_str().to_string() },
        date: QueryDate {
            year: criteria.departure.year(),
            month: criteria.departure.month(),
            day: criteria.departure.day(),
        },
    }];

    if let Some(ret) = criteria.return_date {
        legs.push(QueryLeg {
            origin_place_id: PlaceId { iata: criteria.destination.as_str().to_string() },
            destination_place_id: PlaceId { iata: criteria.origin.as_str().to_string() },
            date: QueryDate { year: ret.year(), month: ret.month(), day: ret.day() },
        });
    }

    let children_ages = vec![8u8; criteria.passengers.children as usize];

    CreateRequest {
        query: Query {
            market: "UK".to_string(),
            locale: "en-GB".to_string(),
            currency: "EUR".to_string(),
            adults: criteria.passengers.adults,
            children_ages,
            cabin_class: cabin_class_str(criteria.cabin).to_string(),
            query_legs: legs,
        },
    }
}

fn cabin_class_str(cabin: CabinClass) -> &'static str {
    match cabin {
        CabinClass::Economy => "CABIN_CLASS_ECONOMY",
        CabinClass::Business => "CABIN_CLASS_BUSINESS",
        CabinClass::First => "CABIN_CLASS_FIRST",
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use serde_json::json;
    use wiremock::{
        matchers::{method, path, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::domain::{
        airport::IataCode,
        flight::value_objects::PassengerCount,
        search::SearchCriteria,
    };

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

    fn incomplete_body() -> serde_json::Value {
        json!({
            "sessionToken": "test-token",
            "status": "RESULT_STATUS_INCOMPLETE",
            "action": "RESULT_ACTION_REPLACED",
            "content": { "results": {} }
        })
    }

    fn complete_body_with_offer() -> serde_json::Value {
        json!({
            "sessionToken": "test-token",
            "status": "RESULT_STATUS_COMPLETE",
            "action": "RESULT_ACTION_REPLACED",
            "content": {
                "results": {
                    "itineraries": {
                        "itin1": {
                            "pricingOptions": [{
                                "price": { "amount": "189990", "unit": "PRICE_UNIT_MILLI" },
                                "agents": ["agent1"]
                            }],
                            "legIds": ["leg1"]
                        }
                    },
                    "legs": {
                        "leg1": {
                            "originPlaceId": "place_mad",
                            "destinationPlaceId": "place_lhr",
                            "departureDateTime": { "year": 2026, "month": 12, "day": 1, "hour": 10, "minute": 0, "second": 0 },
                            "arrivalDateTime":   { "year": 2026, "month": 12, "day": 1, "hour": 12, "minute": 30, "second": 0 },
                            "durationInMinutes": 150,
                            "stopCount": 0,
                            "marketingCarrierIds": ["carrier_ib"],
                            "segmentIds": ["seg1"]
                        }
                    },
                    "segments": {
                        "seg1": {
                            "marketingFlightNumber": "3456",
                            "marketingCarrierId": "carrier_ib"
                        }
                    },
                    "carriers": { "carrier_ib": { "iata": "IB" } },
                    "places": {
                        "place_mad": { "iata": "MAD" },
                        "place_lhr": { "iata": "LHR" }
                    }
                }
            }
        })
    }

    fn complete_body_empty() -> serde_json::Value {
        json!({
            "sessionToken": "test-token",
            "status": "RESULT_STATUS_COMPLETE",
            "content": { "results": {} }
        })
    }

    fn failed_body() -> serde_json::Value {
        json!({ "sessionToken": "test-token", "status": "RESULT_STATUS_FAILED", "content": null })
    }

    #[tokio::test]
    async fn search_returns_offers_when_create_is_complete_immediately() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/create"))
            .respond_with(ResponseTemplate::new(200).set_body_json(complete_body_with_offer()))
            .mount(&server)
            .await;

        let adapter = SkyscannerAdapter::with_base_url("key".into(), server.uri());
        let offers = adapter.search(&make_criteria()).await.unwrap();

        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].outbound.origin.as_str(), "MAD");
        assert_eq!(offers[0].outbound.destination.as_str(), "LHR");
        assert!((offers[0].price.amount() - 189.99).abs() < 0.01);
    }

    #[tokio::test]
    async fn search_polls_until_complete_and_returns_offers() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/create"))
            .respond_with(ResponseTemplate::new(200).set_body_json(incomplete_body()))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path_regex("/poll/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(complete_body_with_offer()))
            .mount(&server)
            .await;

        let adapter = SkyscannerAdapter::with_base_url("key".into(), server.uri());
        let offers = adapter.search(&make_criteria()).await.unwrap();

        assert_eq!(offers.len(), 1);
    }

    #[tokio::test]
    async fn search_returns_empty_vec_when_no_offers_in_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/create"))
            .respond_with(ResponseTemplate::new(200).set_body_json(complete_body_empty()))
            .mount(&server)
            .await;

        let adapter = SkyscannerAdapter::with_base_url("key".into(), server.uri());
        let offers = adapter.search(&make_criteria()).await.unwrap();

        assert!(offers.is_empty());
    }

    #[tokio::test]
    async fn search_returns_error_on_http_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/create"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let adapter = SkyscannerAdapter::with_base_url("key".into(), server.uri());
        let result = adapter.search(&make_criteria()).await;

        assert!(matches!(result, Err(DomainError::ProviderError)));
    }

    #[tokio::test]
    async fn search_returns_error_when_status_is_failed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/create"))
            .respond_with(ResponseTemplate::new(200).set_body_json(failed_body()))
            .mount(&server)
            .await;

        let adapter = SkyscannerAdapter::with_base_url("key".into(), server.uri());
        let result = adapter.search(&make_criteria()).await;

        assert!(matches!(result, Err(DomainError::ProviderError)));
    }
}
