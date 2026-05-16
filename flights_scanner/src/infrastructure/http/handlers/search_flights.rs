use std::sync::Arc;

use axum::{extract::State, Json};

use crate::{
    application::search_flights::SearchFlightsUseCase,
    infrastructure::http::{
        dto::{FlightOfferResponse, SearchRequest},
        error::HttpError,
        AppState,
    },
};

pub async fn search_flights_handler(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<Vec<FlightOfferResponse>>, HttpError> {
    let (criteria, filters) = req.into_criteria_and_filters().map_err(HttpError::from)?;
    let use_case = SearchFlightsUseCase::new(Arc::clone(&state.flight_search_port));
    let offers = use_case.execute(criteria, filters).await.map_err(HttpError::from)?;
    Ok(Json(offers.into_iter().map(FlightOfferResponse::from).collect()))
}
