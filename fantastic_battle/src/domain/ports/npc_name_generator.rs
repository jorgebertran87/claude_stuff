use crate::domain::model::{Player, Theme};

pub trait NpcNameGenerator: Send + Sync + std::fmt::Debug {
    fn generate(&self, theme: &Theme, count: u32) -> Vec<Player>;
}
