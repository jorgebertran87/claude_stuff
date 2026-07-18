use actix_web::{web, HttpResponse};

use crate::container::AppState;
use crate::infrastructure::http::dto::{
    ErrorResponse, InteractResponse, MoveRequest, MoveResponse, NpcResponse, SessionResponse,
};

pub async fn join(state: web::Data<AppState>) -> HttpResponse {
    let session = state.service.join();
    HttpResponse::Ok().json(SessionResponse::from(session))
}

pub async fn get_session(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let session_id = path.into_inner();
    match state.service.get_session(&session_id) {
        Some(session) => HttpResponse::Ok().json(SessionResponse::from(session)),
        None => HttpResponse::NotFound().json(ErrorResponse {
            error: "session not found".to_string(),
        }),
    }
}

pub async fn move_player(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<MoveRequest>,
) -> HttpResponse {
    let session_id = path.into_inner();
    match state.service.move_player(&session_id, body.direction) {
        Ok(position) => HttpResponse::Ok().json(MoveResponse {
            player_position: position.into(),
            player_direction: body.direction,
        }),
        Err(error) => HttpResponse::Conflict().json(ErrorResponse {
            error: format!("{:?}", error),
        }),
    }
}

pub async fn interact(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let session_id = path.into_inner();
    match state.service.interact(&session_id) {
        Some(npc) => HttpResponse::Ok().json(InteractResponse {
            npc: Some(NpcResponse::from(&npc)),
        }),
        None => HttpResponse::Ok().json(InteractResponse { npc: None }),
    }
}
