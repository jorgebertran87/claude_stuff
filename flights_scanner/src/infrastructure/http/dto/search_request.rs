use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{
    application::{
        error::AppError,
        search_flights::{SearchFilters, SortBy},
    },
    domain::{
        airport::IataCode,
        flight::value_objects::{CabinClass, PassengerCount, Price},
        search::SearchCriteria,
    },
};

#[derive(Deserialize)]
pub struct SearchRequest {
    pub origin: String,
    pub destination: String,
    pub departure_date: DateTime<Utc>,
    pub return_date: Option<DateTime<Utc>>,
    pub adults: u8,
    #[serde(default)]
    pub children: u8,
    #[serde(default)]
    pub infants: u8,
    pub cabin_class: CabinClassDto,
    pub max_price: Option<f64>,
    pub max_stops: Option<u8>,
    pub sort_by: Option<SortByDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CabinClassDto {
    Economy,
    Business,
    First,
}

impl From<CabinClassDto> for CabinClass {
    fn from(dto: CabinClassDto) -> Self {
        match dto {
            CabinClassDto::Economy => CabinClass::Economy,
            CabinClassDto::Business => CabinClass::Business,
            CabinClassDto::First => CabinClass::First,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum SortByDto {
    #[default]
    Price,
    Duration,
}

impl From<SortByDto> for SortBy {
    fn from(dto: SortByDto) -> Self {
        match dto {
            SortByDto::Price => SortBy::Price,
            SortByDto::Duration => SortBy::Duration,
        }
    }
}

impl SearchRequest {
    pub fn into_criteria_and_filters(self) -> Result<(SearchCriteria, SearchFilters), AppError> {
        let origin = IataCode::new(&self.origin).map_err(AppError::Domain)?;
        let destination = IataCode::new(&self.destination).map_err(AppError::Domain)?;
        let passengers = PassengerCount::new(self.adults, self.children, self.infants)
            .map_err(AppError::Domain)?;
        let cabin = CabinClass::from(self.cabin_class);
        let criteria = SearchCriteria::new(
            origin,
            destination,
            self.departure_date,
            self.return_date,
            passengers,
            cabin,
        )
        .map_err(AppError::Domain)?;

        let max_price = self
            .max_price
            .map(|amount| Price::new(amount, "EUR"))
            .transpose()
            .map_err(AppError::Domain)?;

        let filters = SearchFilters {
            max_price,
            max_stops: self.max_stops,
            sort_by: self.sort_by.unwrap_or_default().into(),
        };

        Ok((criteria, filters))
    }
}
