use std::sync::Arc;

use flights_scanner::{
    domain::ports::FlightSearchPort,
    infrastructure::{
        adapters::{InMemoryFlightSearchAdapter, SkyscannerAdapter},
        http::{router::create_router, AppState},
    },
};

#[tokio::main]
async fn main() {
    let port: Arc<dyn FlightSearchPort> = match std::env::var("SKYSCANNER_API_KEY") {
        Ok(key) => {
            println!("Using Skyscanner provider");
            Arc::new(SkyscannerAdapter::new(key))
        }
        Err(_) => {
            println!("SKYSCANNER_API_KEY not set — using in-memory provider");
            Arc::new(InMemoryFlightSearchAdapter::new())
        }
    };

    let state = AppState { flight_search_port: port };
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Flights scanner listening on port 3000");
    axum::serve(listener, app).await.unwrap();
}
