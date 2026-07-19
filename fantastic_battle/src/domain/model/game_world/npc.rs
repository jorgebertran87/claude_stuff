use super::{Direction, Position};
use crate::domain::model::BattleOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcStatus {
    Active,
    DefeatedCorrect,
    DefeatedIncorrect,
}

#[derive(Debug, Clone)]
pub struct NpcSpawn {
    pub name: String,
    pub position: Position,
    pub direction: Direction,
}

#[derive(Debug, Clone)]
pub struct Npc {
    name: String,
    position: Position,
    direction: Direction,
    status: NpcStatus,
}

impl Npc {
    pub fn new(name: String, position: Position, direction: Direction) -> Self {
        Self {
            name,
            position,
            direction,
            status: NpcStatus::Active,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn status(&self) -> NpcStatus {
        self.status
    }

    pub fn defeat(&mut self, outcome: BattleOutcome) {
        self.status = match outcome {
            BattleOutcome::Victory => NpcStatus::DefeatedCorrect,
            BattleOutcome::Defeat => NpcStatus::DefeatedIncorrect,
        };
    }
}
