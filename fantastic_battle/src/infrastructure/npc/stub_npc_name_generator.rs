use crate::domain::model::{Player, Theme};
use crate::domain::ports::NpcNameGenerator;

#[derive(Debug)]
pub struct StubNpcNameGenerator {
    names: Vec<String>,
}

impl StubNpcNameGenerator {
    pub fn new(names: Vec<String>) -> Self {
        Self { names }
    }
}

impl NpcNameGenerator for StubNpcNameGenerator {
    fn generate(&self, _theme: &Theme, count: u32) -> Vec<Player> {
        self.names
            .iter()
            .take(count as usize)
            .map(|n| Player::new(n).unwrap())
            .collect()
    }
}
