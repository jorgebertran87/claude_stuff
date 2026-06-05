use std::process::{Command, Stdio};
use std::sync::Arc;

use shaku::Component;

use crate::domain::ports::{GoogleSheetsGateway, SkillCommands};
use crate::infrastructure::token_usage::parse_result_json;

pub fn run_claude_skill(prompt: &str, model: &str, allowed_tools: Option<&str>, context: &str) -> String {
    eprintln!("[{context}: spawning claude, model={model}]");
    let mut cmd = Command::new("claude");
    cmd.args(["--print", "--output-format", "json", "--model", model]);
    if let Some(tools) = allowed_tools {
        cmd.args(["--allowedTools", tools]);
    }
    let mut child = match cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[{context}: failed to spawn claude: {e}]");
            return "Error al obtener la respuesta.".to_string();
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = std::io::Write::write_all(&mut stdin, prompt.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[{context}: wait error: {e}]");
            return "Error al obtener la respuesta.".to_string();
        }
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprintln!("[{context}: claude stderr: {}]", &stderr[..stderr.len().min(500)]);
    }

    if !output.status.success() {
        eprintln!("[{context}: claude exited {:?}]", output.status.code());
        return "Error al obtener la respuesta.".to_string();
    }

    let json_out = String::from_utf8_lossy(&output.stdout);
    parse_result_json(&json_out)
        .map(|u| u.result)
        .unwrap_or_else(|_| "No pude obtener una respuesta de Claude.".to_string())
}

pub fn handle_cuentas(sheets: &dyn GoogleSheetsGateway, model: &str) -> String {
    let data = match sheets.fetch_as_text() {
        Ok(d) => d,
        Err(e) => return e,
    };
    let sheet_name = std::env::var("CUENTAS_SHEET_NAME")
        .unwrap_or_else(|_| "Cuentas Personales".to_string());
    let prompt = format!("/cuentas {sheet_name}\n\n{data}");
    eprintln!("[cuentas: prompt {} bytes]", prompt.len());
    run_claude_skill(&prompt, model, None, "cuentas")
}

pub fn handle_bus(model: &str, stop_code: &str) -> String {
    let prompt = if stop_code.is_empty() {
        "/bus".to_string()
    } else {
        format!("/bus {stop_code}")
    };
    run_claude_skill(&prompt, model, Some("Bash"), "bus")
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
/// Holds the Google Sheets gateway needed by `/cuentas`.
#[derive(Component)]
#[shaku(interface = SkillCommands)]
pub struct ClaudeSkillCommands {
    #[shaku(inject)]
    sheets: Arc<dyn GoogleSheetsGateway>,
}

impl ClaudeSkillCommands {
    pub fn new(sheets: Arc<dyn GoogleSheetsGateway>) -> Self {
        Self { sheets }
    }
}

impl SkillCommands for ClaudeSkillCommands {
    fn bus(&self, model: &str, stop_code: &str) -> String {
        handle_bus(model, stop_code)
    }

    fn cuentas(&self, model: &str) -> String {
        handle_cuentas(self.sheets.as_ref(), model)
    }

    fn volume(&self, arg: &str) -> String {
        handle_volume(arg)
    }

    fn usage_report(&self, log_file: &str) -> String {
        read_usage_report(log_file)
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
