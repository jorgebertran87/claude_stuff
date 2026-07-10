use cucumber::{given, when, then, World};
use voice_assistant::infrastructure::order_handler::load_prompt;

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

#[then(regex = r#"^the prompt contains "(.+)"$"#)]
fn then_contains(world: &mut SearchPromptWorld, needle: String) {
    assert!(
        world.prompt.contains(&needle),
        "prompt should contain \"{needle}\", got:\n{}",
        world.prompt
    );
}

fn main() {
    futures::executor::block_on(
        SearchPromptWorld::run("features/search_prompt_assembly.feature"),
    );
}
