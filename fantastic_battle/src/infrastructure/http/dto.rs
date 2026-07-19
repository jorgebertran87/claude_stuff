use serde::{Deserialize, Serialize};

use crate::domain::model::game_world::{Direction, GameMap, GameSession, Npc, NpcStatus, Position, TileType};

#[derive(Debug, Serialize)]
pub struct SessionResults {
    pub total: u32,
    pub correct: u32,
    pub incorrect: u32,
    pub remaining: u32,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub player_position: PositionResponse,
    pub player_direction: Direction,
    pub npcs: Vec<NpcResponse>,
    pub map: MapResponse,
    pub results: Option<SessionResults>,
}

impl From<GameSession> for SessionResponse {
    fn from(session: GameSession) -> Self {
        let npcs: Vec<NpcResponse> = session.npcs().iter().map(NpcResponse::from).collect();
        let map = MapResponse::from(session.map());
        let total = npcs.len() as u32;
        let correct = npcs
            .iter()
            .filter(|n| n.status == "DefeatedCorrect")
            .count() as u32;
        let incorrect = npcs
            .iter()
            .filter(|n| n.status == "DefeatedIncorrect")
            .count() as u32;
        let remaining = total - correct - incorrect;
        let results = Some(SessionResults {
            total,
            correct,
            incorrect,
            remaining,
        });
        Self {
            id: session.id().to_string(),
            player_position: PositionResponse::from(session.player_position()),
            player_direction: session.player_direction(),
            npcs,
            map,
            results,
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
    pub status: String,
}

fn npc_status_to_string(status: NpcStatus) -> String {
    match status {
        NpcStatus::Active => "Active".to_string(),
        NpcStatus::DefeatedCorrect => "DefeatedCorrect".to_string(),
        NpcStatus::DefeatedIncorrect => "DefeatedIncorrect".to_string(),
    }
}

impl From<&Npc> for NpcResponse {
    fn from(npc: &Npc) -> Self {
        Self {
            name: npc.name().to_string(),
            position: PositionResponse::from(npc.position()),
            direction: npc.direction(),
            status: npc_status_to_string(npc.status()),
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

impl From<&GameMap> for MapResponse {
    fn from(map: &GameMap) -> Self {
        let tiles: Vec<TileEntry> = map
            .tiles
            .iter()
            .map(|(pos, tile_type)| TileEntry {
                x: pos.x(),
                y: pos.y(),
                tile_type: *tile_type,
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
pub struct JoinRequest {
    pub theme: Option<String>,
    pub question_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MoveRequest {
    pub direction: Option<Direction>,
}

#[derive(Debug, Serialize)]
pub struct MoveResponse {
    pub player_position: PositionResponse,
    pub player_direction: Direction,
}

#[derive(Debug, Serialize)]
pub struct InteractResponse {
    pub npc: Option<NpcResponse>,
    pub battle: Option<BattleResponse>,
}

#[derive(Debug, Serialize)]
pub struct BattleResponse {
    pub question: String,
}

#[derive(Debug, Deserialize)]
pub struct BattleAnswerRequest {
    pub answer: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BattleAnswerResponse {
    pub outcome: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
