pub fn detect_intent(order: &str) -> &'static str {
    let lower = order.to_lowercase();
    if lower.contains("bus") || lower.contains("autobús") || lower.contains("autobus")
        || lower.contains("parada") || lower.contains("línea") || lower.contains("linea")
    {
        "bus"
    } else if lower.contains("música") || lower.contains("musica") || lower.contains("spotify")
        || lower.contains("canción") || lower.contains("cancion") || lower.contains("playlist")
        || lower.contains("reproduce") || lower.contains("pon ")
    {
        "music"
    } else if lower.contains("tiempo") || lower.contains("lluvia") || lower.contains("llover")
        || lower.contains("temperatura")
        || lower.contains("calor") || lower.contains("frío") || lower.contains("frio")
        || lower.contains("clima") || lower.contains("sol") || lower.contains("nube")
        || lower.contains("weather")
    {
        "weather"
    } else {
        "search"
    }
}

pub fn strip_frontmatter(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            return rest[end + 4..].trim().to_string();
        }
    }
    content.trim().to_string()
}

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
    eprintln!("[claude: skill '{name}' not found in any candidate path]");
    String::new()
}

pub fn load_prompt(order: &str) -> String {
    let voice_language = std::env::var("VOICE_LANGUAGE").unwrap_or_else(|_| "es".into());
    let lang_rule = format!("Responde en el idioma oficial del país con código '{voice_language}'.");

    let base     = load_skill("claudito");
    let specific = load_skill(detect_intent(order));

    [lang_rule, base, specific]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
