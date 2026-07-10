/// Port for parsing a minesweeper board screenshot and reasoning about it.
pub trait MinesweeperAnalyzer: Send + Sync {
    /// Parse a board screenshot into its textual board representation.
    /// Returns `None` if the board could not be parsed.
    fn parse_board(&self, image: &[u8]) -> Option<String>;
    /// Answer the user's question about a parsed board.
    fn analyze(&self, board: &str, caption: &str, model: &str) -> String;
}
