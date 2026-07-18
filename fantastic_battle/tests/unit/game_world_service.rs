use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use cucumber::{given, then, when, World};

use fantastic_battle::domain::model::game_world::{
    Direction, GameMap, GameSession, MoveError, Npc, NpcSpawn, Position, TileType,
};
use fantastic_battle::domain::ports::{MapRepository, SessionRepository};
use fantastic_battle::domain::service::GameWorldService;

#[derive(Debug)]
struct FakeMapRepository {
    width: i32,
    height: i32,
    start_position: Position,
    walls: HashSet<Position>,
    npc_spawns: Vec<NpcSpawn>,
}

impl FakeMapRepository {
    fn default_map() -> Self {
        Self {
            width: 100,
            height: 100,
            start_position: Position::new(5, 5),
            walls: HashSet::new(),
            npc_spawns: Vec::new(),
        }
    }
}

impl MapRepository for FakeMapRepository {
    fn load(&self) -> GameMap {
        let mut tiles = HashMap::new();
        for y in 0..self.height {
            for x in 0..self.width {
                tiles.insert(Position::new(x, y), TileType::Grass);
            }
        }
        for wall_pos in &self.walls {
            tiles.insert(*wall_pos, TileType::Wall);
        }
        GameMap {
            tiles,
            width: self.width,
            height: self.height,
            start_position: self.start_position,
            npc_spawns: self.npc_spawns.clone(),
        }
    }
}

#[derive(Debug)]
struct FakeSessionRepository {
    sessions: Mutex<HashMap<String, GameSession>>,
}

impl FakeSessionRepository {
    fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl SessionRepository for FakeSessionRepository {
    fn save(&self, session: GameSession) {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.id().to_string(), session);
    }

    fn find(&self, id: &str) -> Option<GameSession> {
        self.sessions.lock().unwrap().get(id).cloned()
    }
}

#[derive(Debug, Default, World)]
pub struct GameWorld {
    map_repo: Option<FakeMapRepository>,
    session_repo: Option<FakeSessionRepository>,
    service: Option<GameWorldService>,
    current_session_id: Option<String>,
    last_position: Option<Position>,
    move_result: Option<Result<Position, MoveError>>,
    interact_result: Option<Option<Npc>>,
}

impl GameWorld {
    fn ensure_service(&mut self) {
        if self.service.is_some() {
            return;
        }
        let map_repo = self
            .map_repo
            .take()
            .unwrap_or_else(FakeMapRepository::default_map);
        let session_repo = self
            .session_repo
            .take()
            .unwrap_or_else(FakeSessionRepository::new);
        self.service = Some(GameWorldService::new(
            Arc::new(map_repo),
            Arc::new(session_repo),
        ));
    }

    fn join_game(&mut self) {
        self.session_repo = Some(FakeSessionRepository::new());
        self.service = None;
        self.ensure_service();
        let service = self.service.as_ref().unwrap();
        let session = service.join();
        self.current_session_id = Some(session.id().to_string());
        self.last_position = Some(session.player_position());
    }
}

// ── Joining the game ─────────────────────────────────────────────────────

#[when("the human player joins the game")]
fn when_join(world: &mut GameWorld) {
    if world.map_repo.is_none() {
        world.map_repo = Some(FakeMapRepository::default_map());
    }
    world.join_game();
}

#[given("the human player has joined the game")]
fn given_joined(world: &mut GameWorld) {
    if world.map_repo.is_none() {
        world.map_repo = Some(FakeMapRepository::default_map());
    }
    world.join_game();
}

#[given("the human player has joined the game on a 3 by 3 map")]
fn given_joined_small(world: &mut GameWorld) {
    world.map_repo = Some(FakeMapRepository {
        width: 3,
        height: 3,
        start_position: Position::new(0, 0),
        walls: HashSet::new(),
        npc_spawns: Vec::new(),
    });
    world.join_game();
}

#[then(regex = r"^the player is at position \((\-?\d+), (\-?\d+)\)$")]
fn then_position(world: &mut GameWorld, x: i32, y: i32) {
    assert_eq!(world.last_position, Some(Position::new(x, y)));
}

#[then("the player is facing south")]
fn then_facing_south(world: &mut GameWorld) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    assert_eq!(session.player_direction(), Direction::South);
}

// ── Movement ──────────────────────────────────────────────────────────────

#[when("the player moves north")]
fn when_move_north(world: &mut GameWorld) {
    world.ensure_service();
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    world.move_result = Some(service.move_player(session_id, Direction::North));
    if let Some(Ok(pos)) = world.move_result {
        world.last_position = Some(pos);
    }
}

#[when("the player moves south")]
fn when_move_south(world: &mut GameWorld) {
    world.ensure_service();
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    world.move_result = Some(service.move_player(session_id, Direction::South));
    if let Some(Ok(pos)) = world.move_result {
        world.last_position = Some(pos);
    }
}

#[when("the player moves east")]
fn when_move_east(world: &mut GameWorld) {
    world.ensure_service();
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    world.move_result = Some(service.move_player(session_id, Direction::East));
    if let Some(Ok(pos)) = world.move_result {
        world.last_position = Some(pos);
    }
}

#[when("the player moves west")]
fn when_move_west(world: &mut GameWorld) {
    world.ensure_service();
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    world.move_result = Some(service.move_player(session_id, Direction::West));
    if let Some(Ok(pos)) = world.move_result {
        world.last_position = Some(pos);
    }
}

#[then("the player is facing north")]
fn then_facing_north(world: &mut GameWorld) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    assert_eq!(session.player_direction(), Direction::North);
}

#[then("the player is facing east")]
fn then_facing_east(world: &mut GameWorld) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    assert_eq!(session.player_direction(), Direction::East);
}

#[then("the player is facing west")]
fn then_facing_west(world: &mut GameWorld) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    assert_eq!(session.player_direction(), Direction::West);
}

// ── Collision ─────────────────────────────────────────────────────────────

#[given(regex = r"^there is a wall at position \((\-?\d+), (\-?\d+)\)$")]
fn given_wall(world: &mut GameWorld, x: i32, y: i32) {
    let repo = world
        .map_repo
        .get_or_insert_with(FakeMapRepository::default_map);
    repo.walls.insert(Position::new(x, y));
    world.join_game();
}

#[then("the move is rejected because the tile is not walkable")]
fn then_rejected_not_walkable(world: &mut GameWorld) {
    let result = world.move_result.as_ref().expect("no move result");
    assert_eq!(*result, Err(MoveError::NotWalkable));
}

#[then("the move is rejected because the position is out of bounds")]
fn then_rejected_out_of_bounds(world: &mut GameWorld) {
    let result = world.move_result.as_ref().expect("no move result");
    assert_eq!(*result, Err(MoveError::OutOfBounds));
}

#[then(regex = r"^the player stays at position \((\-?\d+), (\-?\d+)\)$")]
fn then_stays_at(world: &mut GameWorld, x: i32, y: i32) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    assert_eq!(session.player_position(), Position::new(x, y));
}

// ── NPC Interaction ───────────────────────────────────────────────────────

#[given(regex = r#"^there is an NPC named "(.+)" at position \((\-?\d+), (\-?\d+)\)$"#)]
fn given_npc(world: &mut GameWorld, name: String, x: i32, y: i32) {
    let repo = world
        .map_repo
        .get_or_insert_with(FakeMapRepository::default_map);
    repo.npc_spawns.push(NpcSpawn {
        name,
        position: Position::new(x, y),
        direction: Direction::South,
    });
    world.join_game();
}

#[when("the player interacts")]
fn when_interact(world: &mut GameWorld) {
    world.ensure_service();
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    world.interact_result = Some(service.interact(session_id));
}

#[then(regex = r#"^the interaction returns the NPC named "(.+)"$"#)]
fn then_interact_returns_npc(world: &mut GameWorld, name: String) {
    let result = world.interact_result.as_ref().expect("no interact result");
    let npc = result.as_ref().expect("expected an NPC but got None");
    assert_eq!(npc.name(), &name);
}

#[then("the interaction returns no NPC")]
fn then_interact_returns_none(world: &mut GameWorld) {
    let result = world.interact_result.as_ref().expect("no interact result");
    assert!(result.is_none(), "expected None but got an NPC");
}

fn main() {
    futures::executor::block_on(GameWorld::run(
        "features/game_world_service.feature",
    ));
}
