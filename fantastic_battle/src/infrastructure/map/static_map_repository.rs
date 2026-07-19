use std::collections::HashMap;

use crate::domain::model::game_world::{Direction, GameMap, NpcSpawn, Position, TileType};
use crate::domain::ports::MapRepository;

const TEST_MAP_WIDTH: i32 = 15;
const TEST_MAP_HEIGHT: i32 = 10;

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
        for row in 1..=4 {
            tiles.insert(Position::new(5, row), TileType::Wall);
        }
        for col in 8..=11 {
            tiles.insert(Position::new(col, 6), TileType::Wall);
        }

        GameMap {
            tiles,
            width: TEST_MAP_WIDTH,
            height: TEST_MAP_HEIGHT,
            start_position: Position::new(0, 0),
            npc_spawns: vec![
                NpcSpawn {
                    name: "NPC-1".to_string(),
                    position: Position::new(2, 0),
                    direction: Direction::South,
                },
                NpcSpawn {
                    name: "NPC-2".to_string(),
                    position: Position::new(8, 2),
                    direction: Direction::South,
                },
                NpcSpawn {
                    name: "NPC-3".to_string(),
                    position: Position::new(12, 5),
                    direction: Direction::South,
                },
                NpcSpawn {
                    name: "NPC-4".to_string(),
                    position: Position::new(4, 8),
                    direction: Direction::South,
                },
                NpcSpawn {
                    name: "NPC-5".to_string(),
                    position: Position::new(10, 8),
                    direction: Direction::South,
                },
            ],
        }
    }
}
