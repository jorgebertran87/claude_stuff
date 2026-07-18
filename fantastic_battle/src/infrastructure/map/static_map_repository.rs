use std::collections::HashMap;

use crate::domain::model::game_world::{Direction, GameMap, NpcSpawn, Position, TileType};
use crate::domain::ports::MapRepository;

const TEST_MAP_WIDTH: i32 = 5;
const TEST_MAP_HEIGHT: i32 = 5;

#[derive(Debug)]
pub struct StaticMapRepository;

impl MapRepository for StaticMapRepository {
    fn load(&self) -> GameMap {
        let mut tiles = HashMap::new();
        for y in 0..TEST_MAP_HEIGHT {
            for x in 0..TEST_MAP_WIDTH {
                tiles.insert(Position::new(x, y), TileType::Grass);
            }
        }
        tiles.insert(Position::new(0, 1), TileType::Wall);

        GameMap {
            tiles,
            width: TEST_MAP_WIDTH,
            height: TEST_MAP_HEIGHT,
            start_position: Position::new(0, 0),
            npc_spawns: vec![NpcSpawn {
                name: "Sphinx".to_string(),
                position: Position::new(2, 0),
                direction: Direction::South,
            }],
        }
    }
}
