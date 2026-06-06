//! Domain: turn a command's output into a single Telegram reply.
//!
//! A non-zero exit code is shown as an "[exit N]" line placed first, so it
//! survives truncation of a long body. The body is stdout followed by stderr;
//! when there is no output at all it becomes "(no output)". Replies are capped
//! at Telegram's message limit, replacing the overflowing tail with a marker.
//! Plain text — no parse mode, so no markup escaping is needed.

use crate::executor::CommandOutput;

/// Telegram's maximum message length, in characters.
const TELEGRAM_LIMIT: usize = 4096;

/// Appended in place of the tail when a reply is truncated to fit the limit.
const TRUNCATION_MARKER: &str = "\n[truncated]";

pub fn format(output: &CommandOutput) -> String {
    let mut reply = String::new();

    if output.exit_code != 0 {
        reply.push_str(&format!("[exit {}]\n", output.exit_code));
    }

    let mut body = output.stdout.clone();
    if !output.stderr.is_empty() {
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(&output.stderr);
    }
    if body.is_empty() {
        body.push_str("(no output)");
    }
    reply.push_str(&body);

    truncate(reply)
}

/// Cap the reply at the Telegram limit, replacing the overflowing tail with the
/// marker so the total stays within the limit.
fn truncate(reply: String) -> String {
    if reply.chars().count() <= TELEGRAM_LIMIT {
        return reply;
    }
    let keep = TELEGRAM_LIMIT - TRUNCATION_MARKER.chars().count();
    let head: String = reply.chars().take(keep).collect();
    format!("{head}{TRUNCATION_MARKER}")
}
