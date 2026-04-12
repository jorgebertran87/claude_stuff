use cucumber::{when, then, World};

use voice_assistant::container;
use voice_assistant::domain::model::{Language, WakeWord};

#[derive(Debug, Default, World)]
pub struct ContainerWorld {
    assembled: bool,
}

#[when(regex = r#"^build_voice_service is called with wake word "(.+)" and language "(.+)"$"#)]
fn when_build_voice_service(world: &mut ContainerWorld, wake_word: String, language: String) {
    let wake_word = WakeWord::new(wake_word).unwrap();
    let language  = Language::new(language).unwrap();
    container::build_voice_service(wake_word, language);
    world.assembled = true;
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
