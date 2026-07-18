use serde::{Deserialize, Serialize};

use crate::domain::model::game_world::{Direction, GameMap, GameSession, Npc, Position, TileType};

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub player_position: PositionResponse,
    pub player_direction: Direction,
    pub npcs: Vec<NpcResponse>,
    pub map: MapResponse,
}

impl From<GameSession> for SessionResponse {
    fn from(session: GameSession) -> Self {
        Self {
            id: session.id().to_string(),
            player_position: PositionResponse::from(session.player_position()),
            player_direction: session.player_direction(),
            npcs: vec![], // NPCs live on the map, surfaced via interact
            map: MapResponse::from(session.into_map()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub x: i32,
    pub y: i32,
}

impl From<Position> for PositionResponse {
    fn from(pos: Position) -> Self {
        Self {
            x: pos.x(),
            y: pos.y(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NpcResponse {
    pub name: String,
    pub position: PositionResponse,
    pub direction: Direction,
}

impl From<&Npc> for NpcResponse {
    fn from(npc: &Npc) -> Self {
        Self {
            name: npc.name().to_string(),
            position: PositionResponse::from(npc.position()),
            direction: npc.direction(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MapResponse {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<TileEntry>,
    pub start_position: PositionResponse,
}

#[derive(Debug, Serialize)]
pub struct TileEntry {
    pub x: i32,
    pub y: i32,
    #[serde(rename = "type")]
    pub tile_type: TileType,
}

impl From<GameMap> for MapResponse {
    fn from(map: GameMap) -> Self {
        let tiles: Vec<TileEntry> = map
            .tiles
            .into_iter()
            .map(|(pos, tile_type)| TileEntry {
                x: pos.x(),
                y: pos.y(),
                tile_type,
            })
            .collect();
        Self {
            width: map.width,
            height: map.height,
            tiles,
            start_position: PositionResponse::from(map.start_position),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MoveRequest {
    pub direction: Direction,
}

#[derive(Debug, Serialize)]
pub struct MoveResponse {
    pub player_position: PositionResponse,
    pub player_direction: Direction,
}

#[derive(Debug, Serialize)]
pub struct InteractResponse {
    pub npc: Option<NpcResponse>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
