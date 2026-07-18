mod direction;
mod game_map;
mod game_session;
mod npc;
mod position;
mod tile_type;

pub use direction::Direction;
pub use game_map::GameMap;
pub use game_session::GameSession;
pub use npc::{Npc, NpcSpawn};
pub use position::Position;
pub use tile_type::TileType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    OutOfBounds,
    NotWalkable,
}
