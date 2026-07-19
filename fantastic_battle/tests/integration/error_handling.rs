use actix_web::http::StatusCode;
use actix_web::{test, web, App};
use cucumber::{given, then, when, World};
use serde_json::Value;

use fantastic_battle::container::{self, AppState};
use fantastic_battle::infrastructure::http;

#[derive(Debug, Default, World)]
pub struct ErrorHandlingWorld {
    state: Option<AppState>,
    response_status: Option<StatusCode>,
    response_body: Option<Vec<u8>>,
    session_id: Option<String>,
}

impl ErrorHandlingWorld {
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

async fn call_post(
    state: &AppState,
    uri: &str,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri(uri)
        .set_payload(body.to_string())
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body = test::read_body(resp).await.to_vec();
    (status, body)
}

async fn call_get(state: &AppState, uri: &str) -> (StatusCode, Vec<u8>) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(http::configure),
    )
    .await;
    let req = test::TestRequest::get().uri(uri).to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body = test::read_body(resp).await.to_vec();
    (status, body)
}

async fn create_session(state: &AppState) -> String {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state.clone()))
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
    json["id"].as_str().unwrap().to_string()
}

async fn start_and_finish_battle(state: &AppState, session_id: &str) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state.clone()))
            .configure(http::configure),
    )
    .await;

    let move_uri = format!("/api/sessions/{}/move", session_id);
    let req = test::TestRequest::post()
        .uri(&move_uri)
        .set_payload("{\"direction\":\"East\"}")
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    test::call_service(&app, req).await;

    let interact_uri = format!("/api/sessions/{}/interact", session_id);
    let req = test::TestRequest::post().uri(&interact_uri).to_request();
    test::call_service(&app, req).await;

    let answer_uri = format!("/api/sessions/{}/battle/answer", session_id);
    let req = test::TestRequest::post()
        .uri(&answer_uri)
        .set_payload("{\"answer\":\"Zeus\"}")
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    test::call_service(&app, req).await;
}

#[given("a game session exists")]
async fn given_session_exists(world: &mut ErrorHandlingWorld) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    world.session_id = Some(create_session(state).await);
}

#[given("a game session exists with a finished battle")]
async fn given_session_with_finished_battle(world: &mut ErrorHandlingWorld) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    let id = create_session(state).await;
    start_and_finish_battle(state, &id).await;
    world.session_id = Some(id);
}

#[given("a game session exists with no battle")]
async fn given_session_with_no_battle(world: &mut ErrorHandlingWorld) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    world.session_id = Some(create_session(state).await);
}

#[when(
    regex = r#"^the client sends a POST request to "([^"]+)" with an empty body$"#
)]
async fn when_post_empty_body(world: &mut ErrorHandlingWorld, path_template: String) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    let uri = path_template.replace("{session_id}", world.session_id.as_deref().unwrap_or("nonexistent"));
    let (status, body) = call_post(state, &uri, "{}").await;
    world.response_status = Some(status);
    world.response_body = Some(body);
}

#[when(
    regex = r#"^the client sends a POST request to "([^"]+)" with answer "(.+)"$"#
)]
async fn when_post_with_answer(world: &mut ErrorHandlingWorld, path_template: String, answer: String) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    let uri = path_template.replace("{session_id}", world.session_id.as_deref().unwrap_or("nonexistent"));
    let body = format!("{{\"answer\":\"{}\"}}", answer);
    let (status, body) = call_post(state, &uri, &body).await;
    world.response_status = Some(status);
    world.response_body = Some(body);
}

#[when(
    regex = r#"^the client sends a POST request to "([^"]+)" with direction "(.+)"$"#
)]
async fn when_post_with_direction(world: &mut ErrorHandlingWorld, path_template: String, direction: String) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    let uri = path_template.replace("{session_id}", world.session_id.as_deref().unwrap_or("nonexistent"));
    let body = format!("{{\"direction\":\"{}\"}}", direction);
    let (status, body) = call_post(state, &uri, &body).await;
    world.response_status = Some(status);
    world.response_body = Some(body);
}

#[when(
    regex = r#"^the client sends a POST request to "([^"]+)"$"#
)]
async fn when_post_no_body(world: &mut ErrorHandlingWorld, path_template: String) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    let uri = path_template.replace("{session_id}", world.session_id.as_deref().unwrap_or("nonexistent"));
    let (status, body) = call_post(state, &uri, "").await;
    world.response_status = Some(status);
    world.response_body = Some(body);
}

#[when(
    regex = r#"^the client sends a GET request to "([^"]+)"$"#
)]
async fn when_get(world: &mut ErrorHandlingWorld, path_template: String) {
    world.ensure_state();
    let state = world.state.as_ref().unwrap();
    let uri = path_template.replace("{session_id}", world.session_id.as_deref().unwrap_or("nonexistent"));
    let (status, body) = call_get(state, &uri).await;
    world.response_status = Some(status);
    world.response_body = Some(body);
}

#[then(regex = r#"^the server responds with status (\d+)$"#)]
fn then_status(world: &mut ErrorHandlingWorld, status: u16) {
    let expected = StatusCode::from_u16(status).expect("invalid status code");
    assert_eq!(world.response_status, Some(expected));
}

#[then(regex = r#"^the error message is "(.+)"$"#)]
fn then_error_message(world: &mut ErrorHandlingWorld, message: String) {
    let json = world.json_body();
    assert_eq!(json["error"].as_str().unwrap(), &message);
}

fn main() {
    futures::executor::block_on(ErrorHandlingWorld::run(
        "features/error_handling.feature",
    ));
}
