use sha2::{Digest, Sha256};
use similar::{ChangeTag, TextDiff};
use std::path::Path;

// ---------------------------------------------------------------------------
// File state
// ---------------------------------------------------------------------------

pub struct FileState {
    pub content: String,
    pub hash: String,
}

/// Read the monitored file and compute its SHA-256 hash.
pub fn read_file(path: &Path) -> anyhow::Result<FileState> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Cannot read {:?}: {}", path, e))?;
    let hash = sha256(&content);
    Ok(FileState { content, hash })
}

// ---------------------------------------------------------------------------
// State persistence
// ---------------------------------------------------------------------------

/// Load the previously saved content from the state file.
/// Returns `None` if the state file does not exist yet.
pub fn load_state(state_file: &Path) -> Option<FileState> {
    let content = std::fs::read_to_string(state_file).ok()?;
    let hash = sha256(&content);
    Some(FileState { content, hash })
}

/// Persist the current content so we can diff against it next cycle.
pub fn save_state(state_file: &Path, state: &FileState) -> anyhow::Result<()> {
    // Make sure the parent directory exists (e.g. /data inside the container).
    if let Some(parent) = state_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(state_file, &state.content)
        .map_err(|e| anyhow::anyhow!("Cannot write state to {:?}: {}", state_file, e))
}

// ---------------------------------------------------------------------------
// Diff
// ---------------------------------------------------------------------------

/// Produce a human-readable unified-style diff (only changed lines).
/// Lines are prefixed with `+` (added) or `-` (removed).
/// Truncated to `max_chars` to stay within Telegram's 4 096-char message limit.
pub fn compute_diff(old: &str, new: &str, max_chars: usize) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut out = String::new();

    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "- ",
            ChangeTag::Insert => "+ ",
            ChangeTag::Equal => continue,
        };
        out.push_str(prefix);
        out.push_str(change.as_str().unwrap_or(""));
        if !out.ends_with('\n') {
            out.push('\n');
        }
        if out.len() >= max_chars {
            out.truncate(max_chars);
            out.push_str("\n…(diff truncated)");
            break;
        }
    }

    if out.is_empty() {
        "(no textual diff — possible whitespace or encoding change)".into()
    } else {
        out
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sha256(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}
