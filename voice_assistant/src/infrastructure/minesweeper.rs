use std::time::Duration;

use shaku::Component;

use crate::domain::ports::MinesweeperAnalyzer;
use crate::infrastructure::telegram_skills::run_claude_skill;

pub fn run_minesweeper_parser(bytes: &[u8]) -> Option<String> {
    let base_url = std::env::var("MINESWEEPER_URL")
        .unwrap_or_else(|_| "http://minesweeper:5000".to_string());
    let url = format!("{base_url}/parse");

    let resp = ureq::post(&url)
        .set("Content-Type", "application/octet-stream")
        .timeout(Duration::from_secs(30))
        .send_bytes(bytes)
        .map_err(|e| { eprintln!("[minesweeper: HTTP error: {e}]"); e })
        .ok()?;

    let body = resp.into_string()
        .map_err(|e| eprintln!("[minesweeper: read error: {e}]"))
        .ok()?;

    if body.trim().is_empty() { None } else { Some(body) }
}

pub fn board_to_json(board: &str) -> String {
    let mut mines_remaining: Option<u32> = None;
    let mut flags: Vec<serde_json::Value> = vec![];
    let mut unrevealed: Vec<serde_json::Value> = vec![];
    let mut revealed: Vec<serde_json::Value> = vec![];
    let mut row = 0usize;

    for line in board.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Mines:") {
            mines_remaining = rest.trim().parse().ok();
            continue;
        }
        row += 1;
        for (col, symbol) in trimmed.split_whitespace().enumerate() {
            let col = col + 1;
            let pos = serde_json::json!({ "row": row, "col": col });
            match symbol {
                "⚑" => flags.push(pos),
                "■" => unrevealed.push(pos),
                "·" => revealed.push(serde_json::json!({ "row": row, "col": col, "value": "empty" })),
                n   => revealed.push(serde_json::json!({ "row": row, "col": col, "value": n })),
            }
        }
    }

    let obj = serde_json::json!({
        "mines_remaining": mines_remaining,
        "flags": flags,
        "unrevealed": unrevealed,
        "revealed": revealed,
    });
    serde_json::to_string(&obj).unwrap_or_default()
}

pub fn analyze_minesweeper_board(board: &str, caption: &str, model: &str) -> String {
    let board_json = board_to_json(board);
    let prompt = format!("/minesweeper {board_json}\n\nPregunta del usuario: {caption}");
    eprintln!("[minesweeper: prompt]\n{prompt}");
    run_claude_skill(&prompt, model, Some("Bash,WebSearch"), "minesweeper")
}

// ── MinesweeperService ────────────────────────────────────────────────────────

#[derive(Component)]
#[shaku(interface = MinesweeperAnalyzer)]
pub struct MinesweeperService;

impl MinesweeperAnalyzer for MinesweeperService {
    fn parse_board(&self, image: &[u8]) -> Option<String> {
        run_minesweeper_parser(image)
    }

    fn analyze(&self, board: &str, caption: &str, model: &str) -> String {
        analyze_minesweeper_board(board, caption, model)
    }
}
