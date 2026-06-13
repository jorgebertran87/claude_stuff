use cucumber::{given, then, when, World};
use prices_comparer::token_store::TokenStore;

const TOKEN_FILE: &str = "glovo_token";

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(World)]
pub struct TokenWorld {
    // TempDir must outlive the store so the backing file stays put.
    dir: tempfile::TempDir,
    store: TokenStore,
}

impl std::fmt::Debug for TokenWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenWorld")
            .field("current", &self.store.current())
            .finish()
    }
}

impl Default for TokenWorld {
    fn default() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = TokenStore::new(dir.path().join(TOKEN_FILE));
        Self { dir, store }
    }
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("a fresh token store")]
fn given_fresh(_world: &mut TokenWorld) {}

#[given(regex = r#"^a token store holding "([^"]*)"$"#)]
fn given_holding(world: &mut TokenWorld, token: String) {
    world.store.set(&token).unwrap();
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the token "([^"]*)" is saved$"#)]
fn when_save(world: &mut TokenWorld, token: String) {
    world.store.set(&token).unwrap();
}

#[when("another store opens the same file")]
fn when_reopen(world: &mut TokenWorld) {
    world.store = TokenStore::new(world.dir.path().join(TOKEN_FILE));
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then("the store has no token")]
fn then_no_token(world: &mut TokenWorld) {
    assert_eq!(world.store.current(), None, "expected no token");
}

#[then(regex = r#"^the current token is "([^"]*)"$"#)]
fn then_current(world: &mut TokenWorld, expected: String) {
    assert_eq!(world.store.current(), Some(expected), "token mismatch");
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    TokenWorld::run("features/token_store.feature").await;
}
