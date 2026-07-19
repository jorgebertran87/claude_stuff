use actix_web::{web, HttpResponse};

use crate::container::AppState;
use crate::domain::model::{BattleError, Player, Theme};
use crate::domain::service::GameWorldError;
use crate::infrastructure::http::dto::{
    BattleAnswerRequest, BattleAnswerResponse, BattleResponse, ErrorResponse, InteractResponse,
    MoveRequest, MoveResponse, NpcResponse, SessionResponse,
};

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

pub async fn join(state: web::Data<AppState>) -> HttpResponse {
    let session = state.game_service.join();
    HttpResponse::Ok().json(SessionResponse::from(session))
}

pub async fn get_session(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let session_id = path.into_inner();
    match state.game_service.get_session(&session_id) {
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
    let direction = match body.direction {
        Some(d) => d,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "missing direction field".to_string(),
            })
        }
    };
    match state.game_service.move_player(&session_id, direction) {
        Ok(position) => HttpResponse::Ok().json(MoveResponse {
            player_position: position.into(),
            player_direction: direction,
        }),
        Err(GameWorldError::SessionNotFound) => {
            HttpResponse::NotFound().json(ErrorResponse {
                error: "session not found".to_string(),
            })
        }
        Err(GameWorldError::Move(error)) => {
            HttpResponse::Conflict().json(ErrorResponse {
                error: error.to_string(),
            })
        }
    }
}

pub async fn interact(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let session_id = path.into_inner();
    let npc = match state.game_service.interact(&session_id) {
        Ok(n) => n,
        Err(GameWorldError::SessionNotFound) => {
            return HttpResponse::NotFound().json(ErrorResponse {
                error: "session not found".to_string(),
            })
        }
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "unexpected error".to_string(),
            })
        }
    };
    let (npc_response, battle) = match npc {
        Some(ref n) => {
            let theme = match Theme::new("Greek mythology") {
                Ok(t) => t,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "failed to create battle theme".to_string(),
                    })
                }
            };
            let player = match Player::new(n.name()) {
                Ok(p) => p,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "failed to create battle opponent".to_string(),
                    })
                }
            };
            let battle = state.battle_service.start_battle(&theme, &player);
            state.battle_repo.save(&session_id, battle.clone());
            (
                Some(NpcResponse::from(n)),
                Some(BattleResponse {
                    question: battle.question().text().to_string(),
                }),
            )
        }
        None => (None, None),
    };
    HttpResponse::Ok().json(InteractResponse {
        npc: npc_response,
        battle,
    })
}

pub async fn answer_battle(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<BattleAnswerRequest>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let answer = match &body.answer {
        Some(a) => a.clone(),
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "missing answer field".to_string(),
            })
        }
    };
    match state.battle_repo.find(&session_id) {
        None => HttpResponse::NotFound().json(ErrorResponse {
            error: "battle not found".to_string(),
        }),
        Some(mut battle) => match battle.answer(&answer) {
            Ok(outcome) => {
                state.battle_repo.save(&session_id, battle);
                let outcome_str = match outcome {
                    crate::domain::model::BattleOutcome::Victory => "Victory",
                    crate::domain::model::BattleOutcome::Defeat => "Defeat",
                };
                HttpResponse::Ok().json(BattleAnswerResponse {
                    outcome: outcome_str.to_string(),
                })
            }
            Err(BattleError::AlreadyOver) => {
                HttpResponse::Conflict().json(ErrorResponse {
                    error: BattleError::AlreadyOver.to_string(),
                })
            }
        },
    }
}

pub async fn get_battle(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let session_id = path.into_inner();
    match state.battle_repo.find(&session_id) {
        None => HttpResponse::NotFound().json(ErrorResponse {
            error: "battle not found".to_string(),
        }),
        Some(battle) => HttpResponse::Ok().json(BattleResponse {
            question: battle.question().text().to_string(),
        }),
    }
}
