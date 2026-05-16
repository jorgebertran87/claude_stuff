pub mod dto;
pub mod error;
pub mod handlers;
pub mod router;

use std::sync::Arc;

use crate::domain::ports::FlightSearchPort;

#[derive(Clone)]
pub struct AppState {
    pub flight_search_port: Arc<dyn FlightSearchPort>,
}
