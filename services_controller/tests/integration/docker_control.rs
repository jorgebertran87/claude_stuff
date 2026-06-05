use cucumber::{given, then, when, World};
use serde_json::json;
use services_controller::control::docker::DockerController;
use services_controller::control::ServiceController;
use services_controller::manager::ServiceStatus;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── World ───────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct DockerWorld {
    // MockServer must be kept alive so the mock stays mounted during the test.
    server: Option<MockServer>,
    server_uri: String,
    controller: Option<DockerController>,
    op_result: Option<Result<(), String>>,
    status_result: Option<ServiceStatus>,
}

impl std::fmt::Debug for DockerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockerWorld")
            .field("server_uri", &self.server_uri)
            .finish()
    }
}

impl Default for DockerWorld {
    fn default() -> Self {
        Self {
            server: None,
            server_uri: String::new(),
            controller: None,
            op_result: None,
            status_result: None,
        }
    }
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a mock Docker API that accepts control of container "([^"]+)"$"#)]
async fn given_accepts(world: &mut DockerWorld, container: String) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(format!(r"^/containers/{container}/(start|stop|restart)$")))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    set_server(world, server);
}

#[given(regex = r#"^a mock Docker API reporting container "([^"]+)" as (running|stopped)$"#)]
async fn given_reports(world: &mut DockerWorld, container: String, state: String) {
    let running = state == "running";
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/containers/{container}/json")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "State": { "Running": running, "Status": state }
        })))
        .mount(&server)
        .await;
    set_server(world, server);
}

#[given(regex = r#"^a mock Docker API that fails control of container "([^"]+)"$"#)]
async fn given_fails(world: &mut DockerWorld, container: String) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex(format!(r"^/containers/{container}/(start|stop|restart)$")))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;
    set_server(world, server);
}

#[given("a Docker controller targeting that API")]
fn given_controller(world: &mut DockerWorld) {
    world.controller = Some(DockerController::new(world.server_uri.clone()));
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^I start container "([^"]+)"$"#)]
async fn when_start(world: &mut DockerWorld, container: String) {
    let r = world.controller.as_ref().unwrap().start(&container).await;
    world.op_result = Some(r.map_err(|e| e.to_string()));
}

#[when(regex = r#"^I stop container "([^"]+)"$"#)]
async fn when_stop(world: &mut DockerWorld, container: String) {
    let r = world.controller.as_ref().unwrap().stop(&container).await;
    world.op_result = Some(r.map_err(|e| e.to_string()));
}

#[when(regex = r#"^I query the status of container "([^"]+)"$"#)]
async fn when_status(world: &mut DockerWorld, container: String) {
    match world.controller.as_ref().unwrap().status(&container).await {
        Ok(s) => {
            world.status_result = Some(s);
            world.op_result = Some(Ok(()));
        }
        Err(e) => world.op_result = Some(Err(e.to_string())),
    }
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then("the control call succeeds")]
fn then_ok(world: &mut DockerWorld) {
    let result = world.op_result.as_ref().expect("no control call was made");
    assert!(result.is_ok(), "expected success, got: {result:?}");
}

#[then("the control call fails")]
fn then_err(world: &mut DockerWorld) {
    let result = world.op_result.as_ref().expect("no control call was made");
    assert!(result.is_err(), "expected failure, but it succeeded");
}

#[then(regex = r#"^the reported status is "([^"]+)"$"#)]
fn then_status(world: &mut DockerWorld, expected: String) {
    let status = world.status_result.as_ref().expect("no status was queried");
    assert_eq!(status.as_str(), expected, "status mismatch");
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn set_server(world: &mut DockerWorld, server: MockServer) {
    world.server_uri = server.uri();
    world.server = Some(server);
}

// ── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    DockerWorld::run("features/docker_control.feature").await;
}
