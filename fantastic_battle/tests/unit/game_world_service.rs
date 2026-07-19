use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use cucumber::{given, then, when, World};

use fantastic_battle::domain::model::game_world::{
    Direction, GameMap, GameSession, MoveError, Npc, NpcSpawn, NpcStatus, Position, TileType,
};
use fantastic_battle::domain::model::{BattleOutcome, Player, Theme};
use fantastic_battle::domain::ports::{MapRepository, NpcNameGenerator, SessionRepository};
use fantastic_battle::domain::service::{GameWorldError, GameWorldService};

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

#[derive(Debug, Clone)]
struct FakeNpcNameGenerator {
    names: Vec<String>,
}

impl NpcNameGenerator for FakeNpcNameGenerator {
    fn generate(&self, _theme: &Theme, count: u32) -> Vec<Player> {
        self.names
            .iter()
            .take(count as usize)
            .map(|n| Player::new(n).unwrap())
            .collect()
    }
}

#[derive(Debug, Default, World)]
pub struct GameWorld {
    map_repo: Option<FakeMapRepository>,
    session_repo: Option<FakeSessionRepository>,
    service: Option<GameWorldService>,
    current_session_id: Option<String>,
    last_position: Option<Position>,
    move_result: Option<Result<Position, GameWorldError>>,
    interact_result: Option<Result<Option<Npc>, GameWorldError>>,
    name_generator_names: Vec<String>,
    current_theme: Option<Theme>,
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
        let name_gen = FakeNpcNameGenerator {
            names: self.name_generator_names.clone(),
        };
        self.service = Some(GameWorldService::new(
            Arc::new(map_repo),
            Arc::new(session_repo),
            Arc::new(name_gen),
        ));
    }

    fn join_game(&mut self, theme_name: &str) {
        self.join_game_with_count(theme_name, 5);
    }

    fn join_game_with_count(&mut self, theme_name: &str, count: u32) {
        self.session_repo = Some(FakeSessionRepository::new());
        self.service = None;
        self.ensure_service();
        let theme = Theme::new(theme_name).unwrap();
        self.current_theme = Some(theme.clone());
        let service = self.service.as_ref().unwrap();
        let session = service.join(&theme, count);
        self.current_session_id = Some(session.id().to_string());
        self.last_position = Some(session.player_position());
    }
}

// ── Joining the game ─────────────────────────────────────────────────────

#[when(regex = r#"^the human player joins the game with the theme "([^"]*)"$"#)]
fn when_join_with_theme(world: &mut GameWorld, theme_name: String) {
    if world.map_repo.is_none() {
        world.map_repo = Some(FakeMapRepository::default_map());
    }
    world.join_game(&theme_name);
}

#[when("the human player joins the game")]
fn when_join(world: &mut GameWorld) {
    if world.map_repo.is_none() {
        world.map_repo = Some(FakeMapRepository::default_map());
    }
    world.join_game("Greek mythology");
}

#[given("the human player has joined the game")]
fn given_joined(world: &mut GameWorld) {
    if world.map_repo.is_none() {
        world.map_repo = Some(FakeMapRepository::default_map());
    }
    world.join_game("Greek mythology");
}

#[given(regex = r#"^the human player has joined the game with the theme "([^"]*)"$"#)]
fn given_joined_with_theme(world: &mut GameWorld, theme_name: String) {
    if world.map_repo.is_none() {
        world.map_repo = Some(FakeMapRepository::default_map());
    }
    world.join_game(&theme_name);
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
    world.join_game("Greek mythology");
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

#[then(regex = r#"^the session has the theme "([^"]*)"$"#)]
fn then_session_theme(world: &mut GameWorld, expected: String) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    assert_eq!(session.theme().value(), &expected);
}

#[given(regex = r#"^the NPC name generator provides the names (.+) for the theme "([^"]*)"$"#)]
fn given_name_gen_names(world: &mut GameWorld, names: String, _theme_name: String) {
    world.name_generator_names = names
        .split(", ")
        .map(|n| n.trim().to_string())
        .collect();
    let repo = world
        .map_repo
        .get_or_insert_with(FakeMapRepository::default_map);
    for (i, _) in world.name_generator_names.iter().enumerate() {
        repo.npc_spawns.push(NpcSpawn {
            name: format!("placeholder-{}", i),
            position: Position::new(10 + i as i32, 10),
            direction: Direction::South,
        });
    }
}

#[then(regex = r#"^the session has (\d+) NPCs$"#)]
fn then_session_npc_count(world: &mut GameWorld, count: usize) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    assert_eq!(session.npcs().len(), count);
}

#[then(regex = r#"^the NPCs are named (.+), (.+), and (.+)$"#)]
fn then_npcs_named_three(world: &mut GameWorld, first: String, second: String, third: String) {
    let expected = vec![first.trim(), second.trim(), third.trim()];
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    let actual: Vec<&str> = session.npcs().iter().map(|n| n.name()).collect();
    assert_eq!(actual, expected);
}

#[then(regex = r#"^the NPCs are named ([^,]+) and ([^,]+)$"#)]
fn then_npcs_named_two(world: &mut GameWorld, name1: String, name2: String) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    let actual: Vec<&str> = session.npcs().iter().map(|n| n.name()).collect();
    assert_eq!(actual, vec![name1.trim(), name2.trim()]);
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
    world.join_game("Greek mythology");
}

#[then("the move is rejected because the tile is not walkable")]
fn then_rejected_not_walkable(world: &mut GameWorld) {
    let result = world.move_result.as_ref().expect("no move result");
    assert_eq!(*result, Err(GameWorldError::Move(MoveError::NotWalkable)));
}

#[then("the move is rejected because the position is out of bounds")]
fn then_rejected_out_of_bounds(world: &mut GameWorld) {
    let result = world.move_result.as_ref().expect("no move result");
    assert_eq!(*result, Err(GameWorldError::Move(MoveError::OutOfBounds)));
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
    world.join_game("Greek mythology");
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
    let npc = result.as_ref().expect("expected Ok but got Err").as_ref().expect("expected an NPC but got None");
    assert_eq!(npc.name(), &name);
}

#[then("the interaction returns no NPC")]
fn then_interact_returns_none(world: &mut GameWorld) {
    let result = world.interact_result.as_ref().expect("no interact result");
    let npc = result.as_ref().expect("expected Ok but got Err");
    assert!(npc.is_none(), "expected None but got an NPC");
}

// ── NPC Status ──────────────────────────────────────────────────────────────

#[then(regex = r#"^the NPC "(.+)" has status (.+)$"#)]
fn then_npc_status(world: &mut GameWorld, name: String, status_name: String) {
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    let session = service.get_session(session_id).unwrap();
    let npc = session
        .npcs()
        .iter()
        .find(|n| n.name() == &name)
        .expect("NPC not found");
    let expected = match status_name.as_str() {
        "Active" => NpcStatus::Active,
        "DefeatedCorrect" => NpcStatus::DefeatedCorrect,
        "DefeatedIncorrect" => NpcStatus::DefeatedIncorrect,
        _ => panic!("unknown status: {}", status_name),
    };
    assert_eq!(npc.status(), expected);
}

#[when(regex = r#"^the player defeats the NPC "(.+)" with outcome (.+)$"#)]
fn when_defeat_npc(world: &mut GameWorld, name: String, outcome_name: String) {
    let outcome = match outcome_name.as_str() {
        "Victory" => BattleOutcome::Victory,
        "Defeat" => BattleOutcome::Defeat,
        _ => panic!("unknown outcome: {}", outcome_name),
    };
    world.ensure_service();
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    service
        .defeat_npc(session_id, &name, outcome)
        .expect("failed to defeat NPC");
}

#[given(regex = r#"^there is an NPC named "(.+)" at position \((\-?\d+), (\-?\d+)\) that has been defeated$"#)]
fn given_defeated_npc(world: &mut GameWorld, name: String, x: i32, y: i32) {
    let repo = world
        .map_repo
        .get_or_insert_with(FakeMapRepository::default_map);
    repo.npc_spawns.push(NpcSpawn {
        name: name.clone(),
        position: Position::new(x, y),
        direction: Direction::South,
    });
    world.join_game("Greek mythology");
    world.ensure_service();
    let service = world.service.as_ref().unwrap();
    let session_id = world.current_session_id.as_ref().unwrap();
    service
        .defeat_npc(session_id, &name, BattleOutcome::Defeat)
        .expect("failed to defeat NPC");
}

// ── Question Count ──────────────────────────────────────────────────────────

#[when(regex = r#"^the human player joins the game with the theme "([^"]*)" and (\d+) questions$"#)]
fn when_join_with_theme_and_count(world: &mut GameWorld, theme_name: String, count: u32) {
    if world.map_repo.is_none() {
        world.map_repo = Some(FakeMapRepository::default_map());
    }
    world.join_game_with_count(&theme_name, count);
}

#[given("the map has 3 NPC spawn positions")]
fn given_map_has_3_spawns(world: &mut GameWorld) {
    let repo = world
        .map_repo
        .get_or_insert_with(FakeMapRepository::default_map);
    repo.npc_spawns = vec![
        NpcSpawn {
            name: "placeholder-0".to_string(),
            position: Position::new(10, 10),
            direction: Direction::South,
        },
        NpcSpawn {
            name: "placeholder-1".to_string(),
            position: Position::new(11, 10),
            direction: Direction::South,
        },
        NpcSpawn {
            name: "placeholder-2".to_string(),
            position: Position::new(12, 10),
            direction: Direction::South,
        },
    ];
}

fn main() {
    futures::executor::block_on(GameWorld::run(
        "features/game_world_service.feature",
    ));
}
