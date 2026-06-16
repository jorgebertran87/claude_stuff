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

pub use skill_loader::{load_skill, strip_frontmatter};

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
