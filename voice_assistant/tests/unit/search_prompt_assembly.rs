use cucumber::{given, when, then, World};
use voice_assistant::infrastructure::claude_handler::load_prompt;

#[derive(Debug, Default, World)]
pub struct SearchPromptWorld {
    order: String,
    prompt: String,
}

#[given(regex = r#"^an order containing "(.+)"$"#)]
fn given_order(world: &mut SearchPromptWorld, text: String) {
    world.order = text;
}

#[when("the system prompt is assembled")]
fn when_assemble(world: &mut SearchPromptWorld) {
    world.prompt = load_prompt(&world.order);
}

#[then(regex = r#"^the prompt does not contain "(.+)"$"#)]
fn then_does_not_contain(world: &mut SearchPromptWorld, needle: String) {
    assert!(
        !world.prompt.contains(&needle),
        "prompt should NOT contain \"{needle}\", got:\n{}",
        world.prompt
    );
}

#[then(regex = r#"^the prompt contains "(.+)" or "(.+)" or "(.+)"$"#)]
fn then_contains_any_of(world: &mut SearchPromptWorld, a: String, b: String, c: String) {
    let found = world.prompt.contains(&a)
        || world.prompt.contains(&b)
        || world.prompt.contains(&c);
    assert!(
        found,
        "prompt should contain at least one of \"{a}\" / \"{b}\" / \"{c}\", got:\n{}",
        world.prompt
    );
}

fn main() {
    futures::executor::block_on(
        SearchPromptWorld::run("features/search_prompt_assembly.feature"),
    );
}
