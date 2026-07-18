use actix_web::http::StatusCode;
use actix_web::{test, web, App};
use cucumber::{given, then, when, World};
use serde_json::Value;

use fantastic_battle::container::{self, AppState};
use fantastic_battle::infrastructure::http;

#[derive(Debug, Default, World)]
pub struct ApiWorld {
    state: Option<AppState>,
    response_status: Option<StatusCode>,
    response_body: Option<Vec<u8>>,
    session_id: Option<String>,
}

impl ApiWorld {
    fn ensure_state(&mut self) {
        if self.state.is_none() {
            self.state = Some(container::build_state());
        }
    }

    fn json_body(&self) -> Value {
        serde_json::from_slice(self.response_body.as_ref().expect("no response body"))
            .expect("invalid JSON in response")
    }
}

// ── Joining a game ──────────────────────────────────────────────────────────

#[when("the client sends a POST request to \"/api/sessions\"")]
async fn when_post_sessions(world: &mut ApiWorld) {
    world.ensure_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/sessions")
        .to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

#[then("the response status is 200")]
fn then_status_200(world: &mut ApiWorld) {
    assert_eq!(world.response_status, Some(StatusCode::OK));
}

#[then("the response body contains a session id")]
fn then_contains_session_id(world: &mut ApiWorld) {
    let json = world.json_body();
    let id = json["id"].as_str().expect("response missing 'id' field");
    assert!(!id.is_empty());
    world.session_id = Some(id.to_string());
}

#[then(regex = r#"^the player position is \((\-?\d+), (\-?\d+)\) facing "(.+)"$"#)]
fn then_position_facing(world: &mut ApiWorld, x: i32, y: i32, direction: String) {
    let json = world.json_body();
    assert_eq!(json["player_position"]["x"].as_i64().unwrap(), x as i64);
    assert_eq!(json["player_position"]["y"].as_i64().unwrap(), y as i64);
    assert_eq!(json["player_direction"].as_str().unwrap(), &direction);
}

// ── Given a session exists ─────────────────────────────────────────────────

#[given("a game session exists")]
async fn given_session_exists(world: &mut ApiWorld) {
    world.ensure_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/sessions")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body).unwrap();
    world.session_id = Some(json["id"].as_str().unwrap().to_string());
}

// ── Retrieving a session ────────────────────────────────────────────────────

#[when("the client sends a GET request to that session")]
async fn when_get_that_session(world: &mut ApiWorld) {
    let id = world.session_id.as_ref().expect("no session id").clone();
    let uri = format!("/api/sessions/{}", id);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

#[when(regex = r#"^the client sends a GET request to "(.+)"$"#)]
async fn when_get_path(world: &mut ApiWorld, path: String) {
    world.ensure_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::get().uri(&path).to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

#[then("the response status is 404")]
fn then_status_404(world: &mut ApiWorld) {
    assert_eq!(world.response_status, Some(StatusCode::NOT_FOUND));
}

// ── Movement ────────────────────────────────────────────────────────────────

#[when(
    regex = r#"^the client sends a POST request to that session's move endpoint with direction "(.+)"$"#
)]
async fn when_move(world: &mut ApiWorld, direction: String) {
    let id = world.session_id.as_ref().expect("no session id").clone();
    let uri = format!("/api/sessions/{}/move", id);
    let body = format!("{{\"direction\":\"{}\"}}", direction);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_payload(body)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

#[then("the response status is 409")]
fn then_status_409(world: &mut ApiWorld) {
    assert_eq!(world.response_status, Some(StatusCode::CONFLICT));
}

#[then(regex = r#"^the error message is "(.+)"$"#)]
fn then_error_message(world: &mut ApiWorld, message: String) {
    let json = world.json_body();
    assert_eq!(json["error"].as_str().unwrap(), &message);
}

// ── NPC Interaction ─────────────────────────────────────────────────────────

#[when("the client sends a POST request to that session's interact endpoint")]
async fn when_interact(world: &mut ApiWorld) {
    let id = world.session_id.as_ref().expect("no session id").clone();
    let uri = format!("/api/sessions/{}/interact", id);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::post().uri(&uri).to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

#[then(regex = r#"^the response body contains an NPC named "(.+)"$"#)]
fn then_contains_npc(world: &mut ApiWorld, name: String) {
    let json = world.json_body();
    let npc = &json["npc"];
    assert!(!npc.is_null(), "expected NPC but got null");
    assert_eq!(npc["name"].as_str().unwrap(), &name);
}

#[then("the response body contains no NPC")]
fn then_no_npc(world: &mut ApiWorld) {
    let json = world.json_body();
    assert!(json["npc"].is_null(), "expected null NPC but got a value");
}

fn main() {
    futures::executor::block_on(ApiWorld::run(
        "features/game_world_api.feature",
    ));
}
