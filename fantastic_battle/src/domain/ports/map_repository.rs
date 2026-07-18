use std::fmt::Debug;

use crate::domain::model::game_world::GameMap;

pub trait MapRepository: Debug + Send + Sync {
    fn load(&self) -> GameMap;
}
