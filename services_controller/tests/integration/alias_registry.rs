use cucumber::{given, then, when, World};
use services_controller::registry::ServiceRegistry;

// ── World ───────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct RegistryWorld {
    // Kept alive so the written config file survives for the duration of the test.
    dir: tempfile::TempDir,
    path: std::path::PathBuf,
    load_result: Option<Result<ServiceRegistry, String>>,
}

impl std::fmt::Debug for RegistryWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryWorld")
            .field("path", &self.path)
            .field("loaded", &self.load_result.as_ref().map(|r| r.is_ok()))
            .finish()
    }
}

impl Default for RegistryWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("aliases.yaml");
        Self { dir, path, load_result: None }
    }
}

impl RegistryWorld {
    fn registry(&self) -> &ServiceRegistry {
        self.load_result
            .as_ref()
            .expect("registry was not loaded")
            .as_ref()
            .unwrap_or_else(|e| panic!("registry failed to load: {e}"))
    }

    fn write(&self, contents: &str) {
        std::fs::write(&self.path, contents).unwrap();
    }
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a config file mapping "([^"]+)" to "([^"]+)" and "([^"]+)" to "([^"]+)"$"#)]
fn given_two_mappings(world: &mut RegistryWorld, a1: String, s1: String, a2: String, s2: String) {
    world.write(&format!("{a1}: {s1}\n{a2}: {s2}\n"));
}

#[given(regex = r#"^a config file mapping "([^"]+)" to "([^"]+)"$"#)]
fn given_one_mapping(world: &mut RegistryWorld, alias: String, service: String) {
    world.write(&format!("{alias}: {service}\n"));
}

#[given("no config file exists at the given path")]
fn given_no_file(world: &mut RegistryWorld) {
    world.path = world.dir.path().join("does-not-exist.yaml");
}

#[given("a config file with invalid contents")]
fn given_invalid(world: &mut RegistryWorld) {
    // Unterminated YAML flow sequence — not parseable as a mapping.
    world.write("web: [unterminated\n");
}

// ── When ────────────────────────────────────────────────────────────────────

#[when("I load the registry from that file")]
fn when_load(world: &mut RegistryWorld) {
    world.load_result = Some(ServiceRegistry::load(&world.path).map_err(|e| e.to_string()));
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the registry resolves "([^"]+)" to "([^"]+)"$"#)]
fn then_resolves(world: &mut RegistryWorld, alias: String, expected: String) {
    let resolved = world.registry().resolve(&alias).expect("alias should resolve");
    assert_eq!(resolved, expected, "resolution mismatch");
}

#[then(regex = r#"^resolving "([^"]+)" reports an unknown alias$"#)]
fn then_unknown(world: &mut RegistryWorld, alias: String) {
    let err = world.registry().resolve(&alias).expect_err("expected an unknown alias error");
    assert!(
        err.to_string().contains("unknown alias"),
        "expected an unknown alias error, got: {err}"
    );
}

#[then("loading fails")]
fn then_load_fails(world: &mut RegistryWorld) {
    let result = world.load_result.as_ref().expect("registry was not loaded");
    assert!(result.is_err(), "expected loading to fail, but it succeeded");
}

// ── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    RegistryWorld::run("features/alias_registry.feature").await;
}
