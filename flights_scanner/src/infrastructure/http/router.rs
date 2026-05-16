use axum::{
    routing::{get, post},
    Router,
};

use crate::infrastructure::http::{
    handlers::search_flights::search_flights_handler,
    AppState,
};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/flights/search", post(search_flights_handler))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use axum::{body::Body, http::{Request, StatusCode}};
    use chrono::{Duration, Utc};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{
        application::fakes::FakeFlightSearchAdapter,
        domain::{
            airport::IataCode,
            flight::{
                entity::Flight,
                value_objects::{CabinClass, FlightNumber, Price},
                FlightOffer,
            },
        },
    };

    fn app_with(offers: Vec<FlightOffer>) -> Router {
        create_router(AppState {
            flight_search_port: Arc::new(FakeFlightSearchAdapter::returning(offers)),
        })
    }

    fn app_empty() -> Router {
        create_router(AppState {
            flight_search_port: Arc::new(FakeFlightSearchAdapter::empty()),
        })
    }

    fn app_failing() -> Router {
        create_router(AppState {
            flight_search_port: Arc::new(FakeFlightSearchAdapter::failing()),
        })
    }

    fn make_offer(price: f64) -> FlightOffer {
        let now = Utc::now();
        FlightOffer::new(
            Flight::new(
                FlightNumber::new("IB3456").unwrap(),
                IataCode::new("MAD").unwrap(),
                IataCode::new("LHR").unwrap(),
                now + Duration::days(30),
                now + Duration::days(30) + Duration::minutes(150),
                CabinClass::Economy,
            ),
            None,
            Price::new(price, "EUR").unwrap(),
            5,
        )
    }

    fn search_body() -> String {
        json!({
            "origin": "MAD",
            "destination": "LHR",
            "departure_date": (Utc::now() + Duration::days(30)).to_rfc3339(),
            "adults": 1,
            "cabin_class": "Economy"
        })
        .to_string()
    }

    fn post_search(body: String) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/v1/flights/search")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    // ── Health ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn health_returns_200() {
        let resp = app_empty()
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // ── Happy path ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn search_returns_200_with_offers() {
        let resp = app_with(vec![make_offer(199.0), make_offer(299.0)])
            .oneshot(post_search(search_body()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn response_contains_expected_fields() {
        let resp = app_with(vec![make_offer(199.0)])
            .oneshot(post_search(search_body()))
            .await
            .unwrap();
        let json = body_json(resp).await;
        let offer = &json[0];
        assert!(offer["id"].is_string());
        assert_eq!(offer["outbound"]["origin"], "MAD");
        assert_eq!(offer["outbound"]["destination"], "LHR");
        assert_eq!(offer["price"]["currency"], "EUR");
        assert_eq!(offer["is_round_trip"], false);
    }

    #[tokio::test]
    async fn search_with_max_price_filter_returns_filtered_results() {
        let body = json!({
            "origin": "MAD",
            "destination": "LHR",
            "departure_date": (Utc::now() + Duration::days(30)).to_rfc3339(),
            "adults": 1,
            "cabin_class": "Economy",
            "max_price": 200.0
        })
        .to_string();
        let resp = app_with(vec![make_offer(100.0), make_offer(300.0)])
            .oneshot(post_search(body))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json.as_array().unwrap().len(), 1);
    }

    // ── Error paths ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn search_returns_404_when_no_results() {
        let resp = app_empty()
            .oneshot(post_search(search_body()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let json = body_json(resp).await;
        assert!(json["error"].is_string());
    }

    #[tokio::test]
    async fn search_returns_400_for_invalid_iata_code() {
        let body = json!({
            "origin": "INVALID",
            "destination": "LHR",
            "departure_date": (Utc::now() + Duration::days(30)).to_rfc3339(),
            "adults": 1,
            "cabin_class": "Economy"
        })
        .to_string();
        let resp = app_with(vec![make_offer(199.0)])
            .oneshot(post_search(body))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_returns_400_for_same_origin_and_destination() {
        let body = json!({
            "origin": "MAD",
            "destination": "MAD",
            "departure_date": (Utc::now() + Duration::days(30)).to_rfc3339(),
            "adults": 1,
            "cabin_class": "Economy"
        })
        .to_string();
        let resp = app_with(vec![make_offer(199.0)])
            .oneshot(post_search(body))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_returns_400_for_past_departure_date() {
        let body = json!({
            "origin": "MAD",
            "destination": "LHR",
            "departure_date": (Utc::now() - Duration::days(1)).to_rfc3339(),
            "adults": 1,
            "cabin_class": "Economy"
        })
        .to_string();
        let resp = app_with(vec![make_offer(199.0)])
            .oneshot(post_search(body))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_returns_503_when_provider_fails() {
        let resp = app_failing()
            .oneshot(post_search(search_body()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
