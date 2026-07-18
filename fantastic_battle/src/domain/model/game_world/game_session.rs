use uuid::Uuid;

use super::{Direction, GameMap, MoveError, Npc, Position};

#[derive(Debug, Clone)]
pub struct GameSession {
    id: String,
    map: GameMap,
    player_position: Position,
    player_direction: Direction,
    npcs: Vec<Npc>,
}

impl GameSession {
    pub fn new(map: GameMap) -> Self {
        let player_position = map.start_position;
        let player_direction = Direction::South;
        let npcs = map
            .npc_spawns
            .iter()
            .map(|s| Npc::new(s.name.clone(), s.position, s.direction))
            .collect();
        Self {
            id: Uuid::new_v4().to_string(),
            map,
            player_position,
            player_direction,
            npcs,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn player_position(&self) -> Position {
        self.player_position
    }

    pub fn player_direction(&self) -> Direction {
        self.player_direction
    }

    pub fn move_player(&mut self, direction: Direction) -> Result<Position, MoveError> {
        let target = self.player_position.adjacent(direction);
        if !self.map.is_within_bounds(target) {
            return Err(MoveError::OutOfBounds);
        }
        if !self.map.is_walkable(target) {
            return Err(MoveError::NotWalkable);
        }
        self.player_position = target;
        self.player_direction = direction;
        Ok(target)
    }

    pub fn interact(&self) -> Option<&Npc> {
        let target = self.player_position.adjacent(self.player_direction);
        self.npcs.iter().find(|n| n.position() == target)
    }
}
