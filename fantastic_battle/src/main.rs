use actix_cors::Cors;
use actix_web::{web, App, HttpServer};

use fantastic_battle::container;
use fantastic_battle::infrastructure::http;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = container::build_state();
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(state.clone()))
            .configure(http::configure)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
