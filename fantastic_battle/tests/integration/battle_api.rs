use actix_web::http::StatusCode;
use actix_web::{test, web, App};
use cucumber::{given, then, when, World};
use serde_json::Value;

use fantastic_battle::container::{self, AppState};
use fantastic_battle::infrastructure::http;

#[derive(Debug, Default, World)]
pub struct BattleApiWorld {
    state: Option<AppState>,
    response_status: Option<StatusCode>,
    response_body: Option<Vec<u8>>,
    session_id: Option<String>,
}

impl BattleApiWorld {
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

// ── Given a session exists ─────────────────────────────────────────────────

#[given("a game session exists")]
async fn given_session_exists(world: &mut BattleApiWorld) {
    world.ensure_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/sessions")
        .set_payload(r#"{"theme": "Greek mythology"}"#)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body = test::read_body(resp).await;
    let json: Value = serde_json::from_slice(&body).unwrap();
    world.session_id = Some(json["id"].as_str().unwrap().to_string());
}

// ── Given a battle has been started ────────────────────────────────────────

#[given("a battle has been started for that session")]
async fn given_battle_started(world: &mut BattleApiWorld) {
    let id = world.session_id.as_ref().expect("no session id").clone();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let uri = format!("/api/sessions/{}/move", id);
    let body = "{\"direction\":\"East\"}";
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_payload(body)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let _resp = test::call_service(&app, req).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let uri = format!("/api/sessions/{}/interact", id);
    let req = test::TestRequest::post().uri(&uri).to_request();
    let _resp = test::call_service(&app, req).await;
}

// ── Interact ───────────────────────────────────────────────────────────────

#[when("the client sends a POST request to that session's interact endpoint")]
async fn when_interact(world: &mut BattleApiWorld) {
    let id = world.session_id.as_ref().expect("no session id").clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let uri = format!("/api/sessions/{}/interact", id);
    let req = test::TestRequest::post().uri(&uri).to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

// ── Move ───────────────────────────────────────────────────────────────────

#[when(
    regex = r#"^the client sends a POST request to that session's move endpoint with direction "(.+)"$"#
)]
async fn when_move(world: &mut BattleApiWorld, direction: String) {
    let id = world.session_id.as_ref().expect("no session id").clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let uri = format!("/api/sessions/{}/move", id);
    let body = format!("{{\"direction\":\"{}\"}}", direction);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_payload(body)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

// ── Answer ─────────────────────────────────────────────────────────────────

#[when(regex = r#"^the client answers "(.+)"$"#)]
async fn when_answer(world: &mut BattleApiWorld, answer: String) {
    let id = world.session_id.as_ref().expect("no session id").clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let uri = format!("/api/sessions/{}/battle/answer", id);
    let body = format!("{{\"answer\":\"{}\"}}", answer);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_payload(body)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

#[when("the client sends a POST to answer a battle")]
async fn when_answer_no_battle(world: &mut BattleApiWorld) {
    let id = world.session_id.as_ref().expect("no session id").clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(world.state.clone().unwrap()))
            .configure(http::configure),
    )
    .await;
    let uri = format!("/api/sessions/{}/battle/answer", id);
    let body = "{\"answer\":\"whatever\"}";
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_payload(body)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    world.response_status = Some(resp.status());
    world.response_body = Some(test::read_body(resp).await.to_vec());
}

// ── Status checks ──────────────────────────────────────────────────────────

#[then("the response status is 200")]
fn then_status_200(world: &mut BattleApiWorld) {
    assert_eq!(world.response_status, Some(StatusCode::OK));
}

#[then("the response status is 404")]
fn then_status_404(world: &mut BattleApiWorld) {
    assert_eq!(world.response_status, Some(StatusCode::NOT_FOUND));
}

// ── NPC ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the response body contains an NPC named "(.+)"$"#)]
fn then_contains_npc(world: &mut BattleApiWorld, name: String) {
    let json = world.json_body();
    let npc = &json["npc"];
    assert!(!npc.is_null(), "expected NPC but got null");
    assert_eq!(npc["name"].as_str().unwrap(), &name);
}

// ── Battle ─────────────────────────────────────────────────────────────────

#[then("the response body contains a battle with a question")]
fn then_contains_battle(world: &mut BattleApiWorld) {
    let json = world.json_body();
    let battle = &json["battle"];
    assert!(!battle.is_null(), "expected battle but got null");
    let question = battle["question"]
        .as_str()
        .expect("battle missing 'question' field");
    assert!(!question.is_empty());
}

#[then("the response body contains no battle")]
fn then_no_battle(world: &mut BattleApiWorld) {
    let json = world.json_body();
    assert!(
        json["battle"].is_null(),
        "expected null battle but got a value"
    );
}

// ── Outcome ────────────────────────────────────────────────────────────────

#[then(regex = r#"^the outcome is "(.+)"$"#)]
fn then_outcome(world: &mut BattleApiWorld, expected: String) {
    let json = world.json_body();
    let outcome = json["outcome"]
        .as_str()
        .expect("response missing 'outcome' field");
    assert_eq!(outcome, &expected);
}

fn main() {
    futures::executor::block_on(BattleApiWorld::run("features/battle_api.feature"));
}
