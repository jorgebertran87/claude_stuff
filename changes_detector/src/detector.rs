use sha2::{Digest, Sha256};
use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// The outcome of a single `check` call.
#[derive(Debug)]
pub enum CheckResult {
    /// Content is identical to the last known state — nothing to do.
    NoChange,
    /// No previous state existed; the supplied content was saved as the
    /// initial snapshot. No notification should be sent for this.
    Bootstrapped,
    /// Content changed. `diff` is a human-readable line-level diff.
    Changed { diff: String },
}

/// Tracks the last-known content of a monitored string and detects changes.
///
/// It is completely agnostic about *how* the string was produced — it only
/// knows about hashing, diffing, and persisting state to a file.
pub struct ChangeDetector {
    state: Option<State>,
    state_file: PathBuf,
}

impl ChangeDetector {
    /// Create a detector, loading any previously persisted state from disk.
    pub fn load(state_file: &Path) -> Self {
        let state = std::fs::read_to_string(state_file).ok().map(|content| {
            let hash = sha256(&content);
            State { content, hash }
        });
        Self {
            state,
            state_file: state_file.to_owned(),
        }
    }

    /// Compare `new_content` against the last known state.
    ///
    /// - If content is unchanged → `NoChange` (no I/O).
    /// - If no previous state exists → persists snapshot, returns `Bootstrapped`.
    /// - If content changed → persists new snapshot, returns `Changed { diff }`.
    pub fn check(&mut self, new_content: String) -> anyhow::Result<CheckResult> {
        let new_hash = sha256(&new_content);

        match &self.state {
            // Content unchanged — short-circuit without touching disk.
            Some(prev) if prev.hash == new_hash => return Ok(CheckResult::NoChange),

            Some(prev) => {
                let diff = build_diff(&prev.content, &new_content, 3_000);
                self.persist(&new_content)?;
                self.state = Some(State { content: new_content, hash: new_hash });
                return Ok(CheckResult::Changed { diff });
            }

            None => {
                // First run — snapshot, but don't raise an alarm.
                self.persist(&new_content)?;
                self.state = Some(State { content: new_content, hash: new_hash });
                return Ok(CheckResult::Bootstrapped);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private
    // -----------------------------------------------------------------------

    fn persist(&self, content: &str) -> anyhow::Result<()> {
        if let Some(parent) = self.state_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.state_file, content).map_err(|e| {
            anyhow::anyhow!("Cannot persist state to {:?}: {e}", self.state_file)
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

struct State {
    content: String,
    hash: String,
}

fn sha256(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

/// Build a `+`/`-` prefixed line-level diff, truncated to `max_chars`.
fn build_diff(old: &str, new: &str, max_chars: usize) -> String {
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
