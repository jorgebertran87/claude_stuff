use std::collections::HashMap;

use super::{NpcSpawn, Position, TileType};

#[derive(Debug, Clone)]
pub struct GameMap {
    pub tiles: HashMap<Position, TileType>,
    pub width: i32,
    pub height: i32,
    pub start_position: Position,
    pub npc_spawns: Vec<NpcSpawn>,
}

impl GameMap {
    pub fn tile_at(&self, position: Position) -> Option<TileType> {
        self.tiles.get(&position).copied()
    }

    pub fn is_within_bounds(&self, position: Position) -> bool {
        position.x() >= 0
            && position.x() < self.width
            && position.y() >= 0
            && position.y() < self.height
    }

    pub fn is_walkable(&self, position: Position) -> bool {
        if !self.is_within_bounds(position) {
            return false;
        }
        !matches!(self.tile_at(position), Some(TileType::Wall))
    }
}
