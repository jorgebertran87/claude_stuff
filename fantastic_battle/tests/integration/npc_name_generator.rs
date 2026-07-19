use cucumber::{then, when, World};

use fantastic_battle::container;
use fantastic_battle::domain::model::{Player, Theme};
use fantastic_battle::domain::ports::NpcNameGenerator;

#[derive(Debug, Default, World)]
pub struct NpcNameGeneratorWorld {
    generator: Option<Box<dyn NpcNameGenerator>>,
    first_names: Vec<Player>,
    second_names: Vec<Player>,
    current_names: Vec<Player>,
}

impl NpcNameGeneratorWorld {
    fn ensure_generator(&mut self) {
        if self.generator.is_none() {
            self.generator = Some(container::test_npc_name_generator());
        }
    }
}

#[when(regex = r#"^the game generates (\d+) NPC names for the theme "([^"]*)"$"#)]
fn when_generate_names(
    world: &mut NpcNameGeneratorWorld,
    count: u32,
    theme_name: String,
) {
    world.ensure_generator();
    let generator = world.generator.as_ref().unwrap();
    let theme = Theme::new(&theme_name).unwrap();
    world.first_names = generator.generate(&theme, count);
    world.current_names = world.first_names.clone();
}

#[when(regex = r#"^the game generates (\d+) NPC names for the theme "([^"]*)" again$"#)]
fn when_generate_names_again(
    world: &mut NpcNameGeneratorWorld,
    count: u32,
    theme_name: String,
) {
    let generator = world.generator.as_ref().unwrap();
    let theme = Theme::new(&theme_name).unwrap();
    world.second_names = generator.generate(&theme, count);
}

#[then(regex = r#"^(\d+) names are returned$"#)]
fn then_count_returned(world: &mut NpcNameGeneratorWorld, expected: u32) {
    assert_eq!(world.current_names.len(), expected as usize);
}

#[then("all generated names are non-empty")]
fn then_names_non_empty(world: &mut NpcNameGeneratorWorld) {
    for player in &world.current_names {
        assert!(!player.name().is_empty(), "found empty player name");
    }
}

#[then("both calls return the same names")]
fn then_same_names(world: &mut NpcNameGeneratorWorld) {
    let first: Vec<&str> = world.first_names.iter().map(|p| p.name()).collect();
    let second: Vec<&str> = world.second_names.iter().map(|p| p.name()).collect();
    assert_eq!(first, second, "expected same names from both calls");
}

fn main() {
    futures::executor::block_on(NpcNameGeneratorWorld::run(
        "features/npc_name_generator.feature",
    ));
}
