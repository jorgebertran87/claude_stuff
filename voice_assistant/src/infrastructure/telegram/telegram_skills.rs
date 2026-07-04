use std::process::{Command, Stdio};

use shaku::Component;

use crate::domain::ports::SkillCommands;

pub fn deepseek_skill(system: &str, user: &str, context: &str) -> String {
    let config = deepseek_client::DeepSeekConfig::from_env();
    eprintln!("[{context}: deepseek, model={}]", config.model);
    match deepseek_client::chat(
        &config.base_url,
        &config.api_key,
        &config.model,
        system,
        user,
        config.reasoning_effort.as_deref(),
    ) {
        Ok(resp) => {
            let preview = if resp.content.len() > 200 { &resp.content[..200] } else { &resp.content };
            eprintln!("[{context} response: {preview}]");
            resp.content
        }
        Err(e) => {
            eprintln!("[{context}: deepseek error: {e}]");
            "Error al obtener la respuesta.".to_string()
        }
    }
}

pub fn handle_bus(_model: &str, stop_code: &str) -> String {
    let code = if stop_code.is_empty() { "1071" } else { stop_code };
    let url = format!("https://navega.emtmalaga.es/api/estimaciones?codPar={code}&v=0.23");

    let data = match ureq::get(&url).call() {
        Ok(r) => match r.into_string() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[bus: read response error: {e}]");
                return "Error al leer los datos de la EMT.".to_string();
            }
        }
        Err(e) => {
            eprintln!("[bus: curl error: {e}]");
            return "Error al consultar la EMT.".to_string();
        }
    };

    let system = "Eres un asistente que muestra horarios de autobús de la EMT Málaga. Responde en texto plano, sin formato markdown.".to_string();
    let user = if stop_code.is_empty() {
        format!("Aquí están los datos de la parada 1071. Filtra los resultados por dirección \"Alameda Principal\". Para cada línea, muestra la próxima salida: en minutos si quedan ≤30 min, o la hora exacta si es más tarde. Responde en texto plano.\n\n{data}")
    } else {
        format!("Aquí están los datos de la parada {code}. Muestra todas las líneas y direcciones disponibles. Para cada línea, muestra la próxima salida: en minutos si quedan ≤30 min, o la hora exacta si es más tarde. Responde en texto plano.\n\n{data}")
    };
    eprintln!("[bus: fetched {} bytes from EMT]", data.len());
    deepseek_skill(&system, &user, "bus")
}

pub fn handle_volume(arg: &str) -> String {
    if !arg.is_empty() {
        let vol = if arg.starts_with('+') || arg.starts_with('-') {
            format!("{}%", arg)
        } else {
            format!("{}%", arg.trim_end_matches('%'))
        };
        let ok = Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &vol])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return "Error al ajustar el volumen.".to_string();
        }
    }
    match Command::new("pactl").args(["get-sink-volume", "@DEFAULT_SINK@"]).output() {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            let pct = text.split('/')
                .find(|s| s.trim().ends_with('%'))
                .and_then(|s| s.trim().trim_end_matches('%').trim().parse::<u32>().ok());
            match pct {
                Some(p) => format!("Volumen: {}%", p),
                None    => "Volumen ajustado.".to_string(),
            }
        }
        Err(_) => "Volumen ajustado.".to_string(),
    }
}

pub fn handle_connect_speakers() -> String {
    let mac = match std::env::var("BT_SPEAKER_MAC") {
        Ok(m) if !m.is_empty() => m,
        _ => return "BT_SPEAKER_MAC no configurado en .env.".to_string(),
    };

    // Power on Bluetooth
    let _ = Command::new("bluetoothctl")
        .args(["power", "on"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // Check if already paired; if not, scan and pair
    let paired = Command::new("bluetoothctl")
        .args(["info", &mac])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("Paired: yes"))
        .unwrap_or(false);

    if !paired {
        eprintln!("[connect_speakers: pairing {mac}]");
        let _ = Command::new("bluetoothctl")
            .args(["remove", &mac])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = Command::new("bluetoothctl")
            .args(["scan", "on"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        std::thread::sleep(std::time::Duration::from_secs(3));
        let _ = Command::new("bluetoothctl")
            .args(["pair", &mac])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("bluetoothctl")
            .args(["scan", "off"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    // Trust and connect
    let _ = Command::new("bluetoothctl")
        .args(["trust", &mac])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let connect_ok = Command::new("bluetoothctl")
        .args(["connect", &mac])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !connect_ok {
        return format!("Error al conectar al altavoz {mac}.").to_string();
    }

    // Wait for the connection to settle, then set A2DP profile
    std::thread::sleep(std::time::Duration::from_secs(2));

    let card = Command::new("pactl")
        .args(["list", "short", "cards"])
        .output()
        .ok()
        .and_then(|out| {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .find(|l| l.to_lowercase().contains("bluez"))
                .and_then(|l| l.split_whitespace().next())
                .map(|s| s.to_string())
        });

    if let Some(ref card_name) = card {
        let _ = Command::new("pactl")
            .args(["set-card-profile", card_name, "a2dp_sink"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    // Set default sink to the Bluetooth sink
    let sink = Command::new("pactl")
        .args(["list", "short", "sinks"])
        .output()
        .ok()
        .and_then(|out| {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .find(|l| l.to_lowercase().contains("bluez"))
                .and_then(|l| l.split_whitespace().next())
                .map(|s| s.to_string())
        });

    match sink {
        Some(sink_name) => {
            let _ = Command::new("pactl")
                .args(["set-default-sink", &sink_name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            format!("Altavoz conectado: {sink_name}").to_string()
        }
        None => "Conectado pero no se encontró el sink Bluetooth.".to_string(),
    }
}

pub fn read_usage_report(log_file: &str) -> String {
    let content = match std::fs::read_to_string(log_file) {
        Ok(c) => c,
        Err(_) => return "No hay datos de uso todavía.".to_string(),
    };
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return "No hay datos de uso todavía.".to_string();
    }

    let mut total_cost = 0.0f64;
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut total_cache_creation: u64 = 0;
    let mut total_tokens: u64 = 0;
    let mut max_cost = 0.0f64;
    let mut max_cost_query = String::new();
    let mut count: u64 = 0;

    for line in &lines {
        let cost = match line.find("cost: $") {
            Some(pos) => {
                let s = &line[pos + 7..];
                let end = s.find(' ').unwrap_or(s.len());
                match s[..end].parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => continue,
                }
            }
            None => continue,
        };
        total_cost += cost;
        total_input += parse_token_field(line, "input: ");
        total_output += parse_token_field(line, "output: ");
        total_cache_read += parse_token_field(line, "cache_read: ");
        total_cache_creation += parse_token_field(line, "cache_creation: ");
        total_tokens += parse_token_field(line, "total: ");
        count += 1;
        if cost > max_cost {
            max_cost = cost;
            if let Some(pos) = line.find("Claude order: ") {
                let s = &line[pos + 14..];
                let end = s.find(" | ").unwrap_or(s.len().min(80));
                max_cost_query = s[..end].to_string();
            }
        }
    }

    if count == 0 {
        return "No hay datos de uso todavía.".to_string();
    }

    format!(
        "Uso de tokens — {count} ordenes\n\n\
         Coste total: ${total_cost:.4} USD\n\
         Coste medio: ${:.4} USD\n\n\
         Tokens totales: {total_tokens}\n\
         \x20 Input:          {total_input}\n\
         \x20 Output:         {total_output}\n\
         \x20 Cache read:     {total_cache_read}\n\
         \x20 Cache creation: {total_cache_creation}\n\n\
         Orden mas cara: ${max_cost:.4} USD\n\
         \x20 \"{max_cost_query}\"",
        total_cost / count as f64,
    )
}

fn parse_token_field(line: &str, field: &str) -> u64 {
    match line.find(field) {
        Some(pos) => {
            let s = &line[pos + field.len()..];
            let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
            s[..end].parse::<u64>().unwrap_or(0)
        }
        None => 0,
    }
}

// ── ClaudeSkillCommands ───────────────────────────────────────────────────────

/// Adapter exposing the bot's slash-command skills as a single injected port.
#[derive(Component)]
#[shaku(interface = SkillCommands)]
pub struct ClaudeSkillCommands;

impl SkillCommands for ClaudeSkillCommands {
    fn bus(&self, model: &str, stop_code: &str) -> String {
        handle_bus(model, stop_code)
    }

    fn volume(&self, arg: &str) -> String {
        handle_volume(arg)
    }

    fn usage_report(&self, log_file: &str) -> String {
        read_usage_report(log_file)
    }

    fn connect_speakers(&self) -> String {
        handle_connect_speakers()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_usage_report_returns_no_data_when_file_missing() {
        let report = read_usage_report("/tmp/nonexistent_orders_tokens_test");
        assert_eq!(report, "No hay datos de uso todavía.");
    }

    #[test]
    fn read_usage_report_summarises_log_lines() {
        let path = "/tmp/test_orders_tokens_usage";
        std::fs::write(
            path,
            "Claude order: hola | Tokens used — input: 10, output: 100, cache_read: 500, cache_creation: 50, total: 660 | cost: $0.002000 USD\n\
             Claude order: adios | Tokens used — input: 20, output: 200, cache_read: 1000, cache_creation: 100, total: 1320 | cost: $0.008000 USD\n",
        ).unwrap();
        let report = read_usage_report(path);
        assert!(report.contains("2 ordenes"), "got: {report}");
        assert!(report.contains("0.0100"), "total cost; got: {report}");
        assert!(report.contains("0.0050"), "avg cost; got: {report}");
        assert!(report.contains("1980"), "total tokens; got: {report}");
        assert!(report.contains("adios"), "most expensive query; got: {report}");
        std::fs::remove_file(path).ok();
    }
}
