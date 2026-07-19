use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use actix_web::middleware::Logger;

use fantastic_battle::container;
use fantastic_battle::infrastructure::http;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("0.0.0.0:{}", port);

    let state = container::build_state();
    let server = HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(state.clone()))
            .configure(http::configure)
    })
    .bind(&bind_addr)?;

    log::info!("Server starting on {}", bind_addr);
    server.run().await
}
