use actix_web::{test, web, App, HttpResponse};
use cucumber::{then, when, World};

#[derive(Debug, Default, World)]
pub struct ServerWorld {
    status: Option<u16>,
    body: Option<String>,
}

#[when("the client sends a GET request to \"/health\"")]
async fn when_health_check(world: &mut ServerWorld) {
    let app = test::init_service(App::new().route("/health", web::get().to(health))).await;
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    world.status = Some(resp.status().as_u16());
    world.body = Some(String::from_utf8(test::read_body(resp).await.to_vec()).unwrap());
}

#[then("the server responds with status 200")]
fn then_status_200(world: &mut ServerWorld) {
    assert_eq!(world.status, Some(200));
}

#[then("the response body is \"ok\"")]
fn then_body_ok(world: &mut ServerWorld) {
    assert_eq!(world.body.as_deref(), Some("ok"));
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

fn main() {
    futures::executor::block_on(ServerWorld::run("features/server_health.feature"));
}
