#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TileType {
    Grass,
    Wall,
}
