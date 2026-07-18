use super::{Direction, Position};

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
}

impl Npc {
    pub fn new(name: String, position: Position, direction: Direction) -> Self {
        Self {
            name,
            position,
            direction,
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
}
