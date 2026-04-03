use cucumber::{given, when, then, World};
use voice_assistant::infrastructure::claude_handler::detect_intent;

#[derive(Debug, Default, World)]
pub struct SkillWorld {
    order: String,
    skill: String,
}

#[given(regex = r#"^an order containing "(.+)"$"#)]
fn given_order(world: &mut SkillWorld, text: String) {
    world.order = text;
}

#[when("the system detects the intent")]
fn when_detect(world: &mut SkillWorld) {
    world.skill = detect_intent(&world.order).to_string();
}

#[then(regex = r#"^the selected skill is "(.+)"$"#)]
fn then_skill(world: &mut SkillWorld, expected: String) {
    assert_eq!(world.skill, expected, "order: {:?}", world.order);
}

fn main() {
    futures::executor::block_on(
        SkillWorld::run("features/skill_routing.feature"),
    );
}
