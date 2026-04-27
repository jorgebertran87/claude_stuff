use cucumber::{when, then, World};

use voice_assistant::container;

#[derive(Debug, Default, World)]
pub struct ContainerWorld {
    assembled: bool,
}

#[when("build_telegram_bot is called with an empty token")]
fn when_build_telegram_bot(world: &mut ContainerWorld) {
    container::build_telegram_bot(String::new());
    world.assembled = true;
}

#[then("the container assembled without panicking")]
fn then_assembled(world: &mut ContainerWorld) {
    assert!(world.assembled, "container construction panicked");
}

fn main() {
    futures::executor::block_on(
        ContainerWorld::run("features/container_wiring.feature"),
    );
}
