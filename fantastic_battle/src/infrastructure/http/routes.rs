use actix_web::web;

use crate::infrastructure::http::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/sessions").route(web::post().to(handlers::join)))
        .service(
            web::resource("/api/sessions/{id}").route(web::get().to(handlers::get_session)),
        )
        .service(
            web::resource("/api/sessions/{id}/move")
                .route(web::post().to(handlers::move_player)),
        )
        .service(
            web::resource("/api/sessions/{id}/interact")
                .route(web::post().to(handlers::interact)),
        );
}
