use std::sync::Arc;

use flights_scanner::{
    domain::ports::FlightSearchPort,
    infrastructure::{
        adapters::{InMemoryFlightSearchAdapter, SkyScrapperAdapter, SkyscannerAdapter},
        http::{router::create_router, AppState},
    },
};

#[tokio::main]
async fn main() {
    let nonempty = |k: String| if k.is_empty() { None } else { Some(k) };

    let sky_scrapper_key = std::env::var("SKY_SCRAPPER_API_KEY").ok().and_then(nonempty);
    let skyscanner_key = std::env::var("SKYSCANNER_API_KEY").ok().and_then(nonempty);

    let port: Arc<dyn FlightSearchPort> = match (sky_scrapper_key, skyscanner_key) {
        (Some(key), _) => {
            println!("Using Sky Scrapper provider");
            Arc::new(SkyScrapperAdapter::new(key))
        }
        (_, Some(key)) => {
            println!("Using Skyscanner provider");
            Arc::new(SkyscannerAdapter::new(key))
        }
        _ => {
            println!("No API key set — using in-memory provider");
            Arc::new(InMemoryFlightSearchAdapter::new())
        }
    };

    let state = AppState { flight_search_port: port };
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Flights scanner listening on port 3000");
    axum::serve(listener, app).await.unwrap();
}
