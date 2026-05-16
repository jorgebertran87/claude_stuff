use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    application::error::AppError,
    domain::error::DomainError,
};

pub enum HttpError {
    BadRequest(String),
    NotFound(String),
    ServiceUnavailable(String),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            HttpError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            HttpError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            HttpError::ServiceUnavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<AppError> for HttpError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Domain(DomainError::InvalidIataCode(c)) => {
                HttpError::BadRequest(format!("invalid IATA code: '{c}'"))
            }
            AppError::Domain(DomainError::SameOriginDestination) => {
                HttpError::BadRequest("origin and destination must differ".into())
            }
            AppError::Domain(DomainError::DepartureDateInPast) => {
                HttpError::BadRequest("departure date must be in the future".into())
            }
            AppError::Domain(DomainError::InvalidPassengerCount(m)) => {
                HttpError::BadRequest(m)
            }
            AppError::Domain(DomainError::InvalidPrice) => {
                HttpError::BadRequest("max_price must be positive".into())
            }
            AppError::NoResults => HttpError::NotFound("no flights found matching the criteria".into()),
            _ => HttpError::ServiceUnavailable("search provider unavailable".into()),
        }
    }
}
