/// Strip YAML frontmatter (delimited by `---`) from a markdown skill file.
pub fn strip_frontmatter(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim().to_string();
        }
    }
    content.trim().to_string()
}

/// Load a skill prompt from `.claude/commands/<name>.md`, searching several
/// well-known locations. Returns an empty string if the file isn't found.
pub fn load_skill(name: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let candidates = [
        format!("/app/.claude/commands/{name}.md"),
        format!("{home}/.claude/commands/{name}.md"),
        format!("../.claude/commands/{name}.md"),
        format!(".claude/commands/{name}.md"),
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            return strip_frontmatter(&content);
        }
    }
    eprintln!("[skill '{name}' not found]");
    String::new()
}
