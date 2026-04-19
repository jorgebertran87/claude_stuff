//! Telegram bot adapter for the voice assistant.
//! Provides long-polling access to Telegram messages and routes them through OrderHandler.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use shaku::Component;

use crate::domain::ports::{GoogleSheetsGateway, ImageAnalyzer, OrderHandler, TextSynthesizer};
use crate::infrastructure::claude_handler::ClaudeImageAnalyzer;
use crate::infrastructure::google_sheets::GoogleSheetsGatewayImpl;
use crate::infrastructure::speaker::GttsTextSynthesizer;

const MODEL_SONNET: &str = "claude-sonnet-4-6";
const MODEL_OPUS:   &str = "claude-opus-4-6";

/// A single Telegram update containing message text and optional photo.
#[derive(Clone)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub chat_id: i64,
    /// Message text or caption (empty string when only a photo was sent).
    pub text: String,
    /// file_id of the largest available photo, if the message contains an image.
    pub photo_file_id: Option<String>,
}

/// Injectable HTTP gateway for Telegram API calls.
/// Separated for testability (real impl uses ureq, tests use FakeGateway).
pub trait TelegramGateway: Send + Sync {
    fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate>;
    fn post_message(&self, chat_id: i64, text: &str);
    /// Send an audio file as a voice message (MP3 bytes).
    fn send_voice(&self, chat_id: i64, data: &[u8]);
    /// Download a file from Telegram by its file_id. Returns raw bytes or None on error.
    fn download_file(&self, file_id: &str) -> Option<Vec<u8>>;
}

/// Real Telegram gateway using ureq HTTP client.
#[derive(Component)]
#[shaku(interface = TelegramGateway)]
pub struct UreqGateway {
    token: String,
}

impl UreqGateway {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    fn base_url(&self) -> String {
        format!("https://api.telegram.org/bot{}", self.token)
    }

    fn split_message(text: &str, max_len: usize) -> Vec<&str> {
        let mut chunks = Vec::new();
        let mut start = 0;
        while start < text.len() {
            let end = (start + max_len).min(text.len());
            // Walk back to a char boundary
            let end = text[..end]
                .char_indices()
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(end);
            chunks.push(&text[start..end]);
            start = end;
        }
        chunks
    }
}

impl TelegramGateway for UreqGateway {
    fn fetch_updates(&self, offset: i64) -> Vec<TelegramUpdate> {
        let url = format!(
            "{}/getUpdates?offset={}&timeout=30",
            self.base_url(),
            offset
        );
        let resp = match ureq::get(&url)
            .timeout(Duration::from_secs(40))
            .call()
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[telegram get_updates error: {e}]");
                return vec![];
            }
        };

        let body = match resp.into_string() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[telegram body read error: {e}]");
                return vec![];
            }
        };

        parse_updates(&body)
    }

    fn post_message(&self, chat_id: i64, text: &str) {
        // Telegram message limit is 4096 chars; split if needed
        for chunk in Self::split_message(text, 4096) {
            let url = format!("{}/sendMessage", self.base_url());
            let json_text = serde_json::to_string(chunk)
                .unwrap_or_else(|_| "\"error sending message\"".into());
            let body = format!(r#"{{"chat_id": {}, "text": {}}}"#, chat_id, json_text);

            if let Err(e) = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body)
            {
                eprintln!("[telegram send_message error: {e}]");
            }
        }
    }

    fn download_file(&self, file_id: &str) -> Option<Vec<u8>> {
        let url = format!("{}/getFile?file_id={}", self.base_url(), file_id);
        let resp = ureq::get(&url)
            .timeout(Duration::from_secs(30))
            .call()
            .ok()?;
        let body = resp.into_string().ok()?;
        let v: Value = serde_json::from_str(&body).ok()?;
        let file_path = v["result"]["file_path"].as_str()?;
        let file_url = format!("https://api.telegram.org/file/bot{}/{}", self.token, file_path);
        let resp = ureq::get(&file_url)
            .timeout(Duration::from_secs(60))
            .call()
            .ok()?;
        let mut bytes = Vec::new();
        resp.into_reader().read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    fn send_voice(&self, chat_id: i64, data: &[u8]) {
        let url = format!("{}/sendVoice", self.base_url());
        let boundary = "TelegramVoiceBoundary";

        let mut body: Vec<u8> = Vec::new();
        // chat_id field
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"chat_id\"\r\n\r\n");
        body.extend_from_slice(chat_id.to_string().as_bytes());
        body.extend_from_slice(b"\r\n");
        // voice field
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"voice\"; filename=\"voice.mp3\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: audio/mpeg\r\n\r\n");
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        if let Err(e) = ureq::post(&url)
            .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .send_bytes(&body)
        {
            eprintln!("[telegram send_voice error: {e}]");
        }
    }
}

fn run_minesweeper_parser(bytes: &[u8]) -> Option<String> {
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

fn analyze_minesweeper_board(board: &str, caption: &str) -> String {
    let prompt = format!("/minesweeper {board}\n\nPregunta del usuario: {caption}");

    eprintln!("[minesweeper analyze: spawning claude, prompt {} bytes]", prompt.len());
    let mut child = match Command::new("claude")
        .args(["--print", "--output-format", "json", "--model", MODEL_SONNET,
               "--allowedTools", "Bash,WebSearch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[minesweeper analyze: failed to spawn claude: {e}]");
            return String::new();
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = std::io::Write::write_all(&mut stdin, prompt.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[minesweeper analyze: wait error: {e}]");
            return String::new();
        }
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprintln!("[minesweeper analyze: claude stderr: {}]", &stderr[..stderr.len().min(500)]);
    }

    if !output.status.success() {
        eprintln!("[minesweeper analyze: claude exited {:?}]", output.status.code());
        return String::new();
    }

    let json_out = String::from_utf8_lossy(&output.stdout);
    eprintln!("[minesweeper analyze: raw json {}]", &json_out[..json_out.len().min(300)]);
    crate::infrastructure::claude_handler::parse_result_json(&json_out)
        .map(|u| u.result)
        .unwrap_or_default()
}

fn play_audio_bytes(bytes: &[u8]) {
    let tmp = "/tmp/tts_telegram_play.mp3";
    if let Err(e) = std::fs::write(tmp, bytes) {
        eprintln!("[play_audio_bytes: failed to write tmp file: {e}]");
        return;
    }
    match Command::new("ffplay")
        .args(["-nodisp", "-autoexit", "-loglevel", "warning", tmp])
        .stdout(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("[play_audio_bytes: ffplay exited with {status}]"),
        Err(e) => eprintln!("[play_audio_bytes: failed to spawn ffplay: {e}]"),
    }
}

/// Main Telegram bot orchestrator.
pub struct TelegramBot {
    gateway: Arc<dyn TelegramGateway>,
    sheets: Arc<dyn GoogleSheetsGateway>,
    synthesizer: Arc<dyn TextSynthesizer>,
    image_analyzer: Arc<dyn ImageAnalyzer>,
    allowed_chat_ids: Vec<i64>,
}

impl TelegramBot {
    /// Create a new Telegram bot using the real ureq HTTP client.
    /// Reads allowed chat IDs from the TELEGRAM_ALLOWED_CHAT_IDS env var.
    pub fn new(token: String) -> Self {
        let allowed: Vec<i64> = std::env::var("TELEGRAM_ALLOWED_CHAT_IDS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();

        if allowed.is_empty() {
            eprintln!("[telegram bot initializing with token (hidden), allowed chats: all (no filter)]");
        } else {
            eprintln!("[telegram bot initializing with token (hidden), allowed chats: {:?}]", allowed);
        }

        Self {
            gateway: Arc::new(UreqGateway::new(token)),
            sheets: Arc::new(GoogleSheetsGatewayImpl),
            synthesizer: Arc::new(GttsTextSynthesizer),
            image_analyzer: Arc::new(ClaudeImageAnalyzer),
            allowed_chat_ids: allowed,
        }
    }

    /// Create a new bot with injectable dependencies (for testing).
    pub fn with_injectable(
        gateway: Arc<dyn TelegramGateway>,
        sheets: Arc<dyn GoogleSheetsGateway>,
        synthesizer: Arc<dyn TextSynthesizer>,
        image_analyzer: Arc<dyn ImageAnalyzer>,
        allowed_chat_ids: Vec<i64>,
    ) -> Self {
        Self {
            gateway,
            sheets,
            synthesizer,
            image_analyzer,
            allowed_chat_ids,
        }
    }

    fn is_allowed(&self, chat_id: i64) -> bool {
        self.allowed_chat_ids.is_empty() || self.allowed_chat_ids.contains(&chat_id)
    }

    fn spawn_analysis(
        gateway: Arc<dyn TelegramGateway>,
        analyzer: Arc<dyn ImageAnalyzer>,
        chat_id: i64,
        file_id: String,
        caption: String,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let lower = caption.to_lowercase();
            let model = if lower.contains("use opus") || lower.contains("utiliza opus") {
                MODEL_OPUS
            } else {
                MODEL_SONNET
            };
            eprintln!("[telegram chat={} image={} model={}]", chat_id, file_id, model);
            let bytes = match gateway.download_file(&file_id) {
                Some(b) => b,
                None => {
                    gateway.post_message(chat_id, "No se pudo descargar la imagen.");
                    return;
                }
            };
            let response = analyzer.analyze(&bytes, &caption, model);
            gateway.post_message(chat_id, &response);
        })
    }

    /// Process one batch of updates from the API.
    /// Split out for testability.
    ///
    /// `voice_mode_chats` tracks which chat IDs have voice mode enabled.
    /// `speak_text` is called with the response text when voice mode is active —
    /// injectable so tests can verify calls without real TTS/audio.
    /// `on_voice` is called whenever audio is actually played; used to reset
    /// the inactivity timer that disconnects the Bluetooth speaker.
    pub fn run_once(
        &self,
        make_handler: &dyn Fn() -> Arc<dyn OrderHandler>,
        handlers: &mut HashMap<i64, Arc<dyn OrderHandler>>,
        voice_mode_chats: &mut HashSet<i64>,
        pending_auth_chats: &mut HashMap<i64, Instant>,
        pending_image_chats: &mut HashMap<i64, String>,
        offset: &mut i64,
        speak_text: &dyn Fn(&str),
        on_voice: &dyn Fn(),
    ) -> Vec<thread::JoinHandle<()>> {
        let mut handles = Vec::new();
        let updates = self.gateway.fetch_updates(*offset);

        for update in updates {
            // Always advance offset, even if we reject the message
            *offset = update.update_id + 1;

            if !self.is_allowed(update.chat_id) {
                eprintln!("[telegram: ignoring unauthorised chat {}]", update.chat_id);
                continue;
            }

            // Handle image/document messages
            if let Some(ref file_id) = update.photo_file_id {
                let caption = update.text.trim();
                if caption.is_empty() {
                    // No caption: store file_id and ask what to do
                    pending_image_chats.insert(update.chat_id, file_id.clone());
                    self.gateway.post_message(update.chat_id, "¿Qué quieres que haga con esta imagen?");
                } else {
                    let lower = caption.to_lowercase();
                    let handle = if lower.contains("buscaminas") || lower.contains("minesweeper") {
                        Self::spawn_minesweeper_analysis(
                            Arc::clone(&self.gateway),
                            update.chat_id,
                            file_id.clone(),
                            caption.to_string(),
                        )
                    } else {
                        Self::spawn_analysis(
                            Arc::clone(&self.gateway),
                            Arc::clone(&self.image_analyzer),
                            update.chat_id,
                            file_id.clone(),
                            caption.to_string(),
                        )
                    };
                    handles.push(handle);
                }
                continue;
            }

            let text = update.text.trim();

            // If a previous image is waiting for a description, use this text as the caption
            if let Some(file_id) = pending_image_chats.remove(&update.chat_id) {
                if !text.starts_with('/') {
                    let lower_text = text.to_lowercase();
                    let handle = if lower_text.contains("buscaminas") || lower_text.contains("minesweeper") {
                        Self::spawn_minesweeper_analysis(
                            Arc::clone(&self.gateway),
                            update.chat_id,
                            file_id,
                            text.to_string(),
                        )
                    } else {
                        Self::spawn_analysis(
                            Arc::clone(&self.gateway),
                            Arc::clone(&self.image_analyzer),
                            update.chat_id,
                            file_id,
                            text.to_string(),
                        )
                    };
                    handles.push(handle);
                    continue;
                }
                // Command received while image pending: put it back and fall through
                pending_image_chats.insert(update.chat_id, file_id);
            }

            // Handle /list — show all available commands
            if text == "/list" {
                self.gateway.post_message(update.chat_id, "\
/list         — muestra este mensaje\n\
/reset        — reinicia la sesión de Claude\n\
/usage        — resumen de tokens y coste acumulado\n\
/voice_mode   — activa/desactiva respuestas por voz\n\
/volume [+N|-N|N] — consulta o ajusta el volumen del altavoz\n\
/cuentas      — analiza tu hoja de cálculo de Google Sheets\n\
/auth_google  — inicia el flujo OAuth2 para Google Sheets");
                continue;
            }

            // Handle pending OAuth2 code exchange (expires after 10 minutes)
            if let Some(&started) = pending_auth_chats.get(&update.chat_id) {
                if !text.starts_with('/') {
                    pending_auth_chats.remove(&update.chat_id);
                    let msg = if started.elapsed() > Duration::from_secs(600) {
                        "El código ha expirado. Usa /auth_google para obtener uno nuevo.".to_string()
                    } else {
                        match self.sheets.exchange_code(text) {
                            Ok(()) => "Token de Google guardado. Ya puedes usar /cuentas.".to_string(),
                            Err(e) => format!("Error al intercambiar el código: {e}"),
                        }
                    };
                    self.gateway.post_message(update.chat_id, &msg);
                    continue;
                }
            }

            // Handle /auth_google — start OAuth2 flow for Google Sheets
            if text == "/auth_google" {
                let msg = match self.sheets.auth_url() {
                    Some(url) => {
                        pending_auth_chats.insert(update.chat_id, Instant::now());
                        format!("Abre este enlace en tu navegador y autoriza el acceso:\n\n{url}\n\nCuando Google te muestre el código, envíamelo aquí.")
                    }
                    None => "GOOGLE_CLIENT_ID no configurado en .env.".to_string(),
                };
                self.gateway.post_message(update.chat_id, &msg);
                continue;
            }

            // Handle /reset command
            if text == "/reset" {
                if let Some(handler) = handlers.get(&update.chat_id) {
                    handler.reset_session();
                }
                self.gateway
                    .post_message(update.chat_id, "Sesión reiniciada.");
                continue;
            }

            // Handle /usage command
            if text == "/usage" {
                let report = read_usage_report(".orders_tokens");
                self.gateway.post_message(update.chat_id, &report);
                continue;
            }

            // Handle /voice_mode command — toggle voice mode for this chat
            if text == "/voice_mode" {
                let enabled = if voice_mode_chats.contains(&update.chat_id) {
                    voice_mode_chats.remove(&update.chat_id);
                    false
                } else {
                    voice_mode_chats.insert(update.chat_id);
                    true
                };
                let msg = if enabled { "Modo voz activado." } else { "Modo voz desactivado." };
                self.gateway.post_message(update.chat_id, msg);
                continue;
            }

            if text == "/cuentas" {
                let handler = handlers.entry(update.chat_id).or_insert_with(make_handler);
                let msg = handle_cuentas(Arc::clone(handler), self.sheets.as_ref());
                self.gateway.post_message(update.chat_id, &msg);
                continue;
            }

            // Handle /volume [+N | -N | N] — adjust or report speaker volume
            if text.starts_with("/volume") {
                let arg = text["/volume".len()..].trim();
                let msg = handle_volume(arg);
                self.gateway.post_message(update.chat_id, &msg);
                continue;
            }

            // Skip other /commands
            if text.starts_with('/') {
                continue;
            }

            // Get or create handler for this chat
            let handler = handlers
                .entry(update.chat_id)
                .or_insert_with(make_handler);

            eprintln!(
                "[telegram chat={} text={:?}]",
                update.chat_id,
                &update.text[..update.text.len().min(50)]
            );
            let response = handler.handle(&update.text);
            let lower = response.to_lowercase();
            let is_alexa_spotify = lower.contains("alexa") && lower.contains("spotify");
            if is_alexa_spotify {
                eprintln!("[telegram: alexa+spotify detected, synthesizing voice order]");
                let bytes = self.synthesizer.synthesize_alexa_spotify(&response);
                if bytes.is_empty() {
                    eprintln!("[telegram: TTS synthesis failed]");
                } else {
                    play_audio_bytes(&bytes);
                    on_voice();
                }
            }
            if voice_mode_chats.contains(&update.chat_id) && !is_alexa_spotify {
                eprintln!("[telegram: voice mode active for chat {}, speaking response]", update.chat_id);
                speak_text(&response);
                on_voice();
            }
            self.gateway.post_message(update.chat_id, &response);
        }
        handles
    }

    fn spawn_minesweeper_analysis(
        gateway: Arc<dyn TelegramGateway>,
        chat_id: i64,
        file_id: String,
        caption: String,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            eprintln!("[minesweeper: chat={} image={}]", chat_id, file_id);
            let bytes = match gateway.download_file(&file_id) {
                Some(b) => b,
                None => {
                    gateway.post_message(chat_id, "No se pudo descargar la imagen.");
                    return;
                }
            };

            gateway.post_message(chat_id, "Analizando tablero de buscaminas...");

            let json = match run_minesweeper_parser(&bytes) {
                Some(j) => j,
                None => {
                    gateway.post_message(
                        chat_id,
                        "No pude parsear el tablero. Asegúrate de que el servicio \
                         minesweeper está corriendo (`docker compose up`).",
                    );
                    return;
                }
            };

            eprintln!("[minesweeper: board]\n{json}");
            gateway.post_message(chat_id, &json);

            let response = analyze_minesweeper_board(&json, &caption);
            let msg = if response.trim().is_empty() {
                "No pude obtener una respuesta de Claude.".to_string()
            } else {
                response
            };
            gateway.post_message(chat_id, &msg);
        })
    }

    /// Main event loop: fetch updates and process them indefinitely.
    pub fn run(&self, make_handler: impl Fn() -> Arc<dyn OrderHandler>) {
        use std::sync::Mutex;
        use std::time::Instant;

        eprintln!("[telegram bot starting, allowed chats: {:?}]", self.allowed_chat_ids);
        let mut offset: i64 = 0;
        let mut handlers: HashMap<i64, Arc<dyn OrderHandler>> = HashMap::new();
        let mut voice_mode_chats: HashSet<i64> = HashSet::new();
        let mut pending_auth_chats: HashMap<i64, Instant> = HashMap::new();
        let mut pending_image_chats: HashMap<i64, String> = HashMap::new();

        // None = voice never used this session; Some(t) = time of last audio playback.
        let last_voice: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let last_voice_bg = Arc::clone(&last_voice);

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(30));
                let elapsed = last_voice_bg.lock().unwrap()
                    .map(|t| t.elapsed())
                    .unwrap_or(Duration::ZERO);
                if elapsed >= Duration::from_secs(300) {
                    crate::infrastructure::speaker::disconnect_bt_speaker();
                    *last_voice_bg.lock().unwrap() = None;
                }
            }
        });

        let synthesizer = Arc::clone(&self.synthesizer);
        let speak_text = move |text: &str| {
            eprintln!("[voice_mode: synthesizing {} chars]", text.len());
            let bytes = synthesizer.synthesize_text(text);
            if bytes.is_empty() {
                eprintln!("[voice_mode: synthesis returned empty bytes, skipping playback]");
            } else {
                eprintln!("[voice_mode: playing {} bytes]", bytes.len());
                play_audio_bytes(&bytes);
            }
        };

        let on_voice = || {
            *last_voice.lock().unwrap() = Some(Instant::now());
        };

        loop {
            self.run_once(&make_handler, &mut handlers, &mut voice_mode_chats, &mut pending_auth_chats, &mut pending_image_chats, &mut offset, &speak_text, &on_voice);
        }
    }
}

fn handle_cuentas(handler: Arc<dyn OrderHandler>, sheets: &dyn GoogleSheetsGateway) -> String {
    let data = match sheets.fetch_as_text() {
        Ok(d) => d,
        Err(e) => return e,
    };

    let sheet_name = std::env::var("CUENTAS_SHEET_NAME")
        .unwrap_or_else(|_| "Cuentas Personales".to_string());

    let prompt = format!(
        "Analiza estos datos de mi hoja de cálculo '{sheet_name}' y dame un resumen claro y detallado:\n\n\
         {data}\n\n\
         Incluye: saldo total por cuenta, ingresos y gastos del período, categorías de gasto principales, \
         y cualquier observación relevante sobre el estado financiero."
    );

    handler.handle(&prompt)
}


/// Set or query the default PulseAudio sink volume via `pactl`.
///
/// `arg` forms:
///   ""      → report current volume
///   "70"    → set to 70 %
///   "+10"   → increase by 10 %
///   "-15"   → decrease by 15 %
fn handle_volume(arg: &str) -> String {
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
    // Report current level
    match Command::new("pactl").args(["get-sink-volume", "@DEFAULT_SINK@"]).output() {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            // Output: "Volume: front-left: 46000 /  70% / -8.58 dB, ..."
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

/// Read and summarise the .orders_tokens log file.
fn read_usage_report(log_file: &str) -> String {
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

/// Parse Telegram API response JSON into updates.
/// Uses serde_json::Value for dynamic parsing (avoids complex derive hierarchies).
fn parse_updates(body: &str) -> Vec<TelegramUpdate> {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[telegram json parse error: {e}]");
            return vec![];
        }
    };

    let mut results = Vec::new();
    let updates = match v["result"].as_array() {
        Some(a) => a,
        None => return vec![],
    };

    for update in updates {
        // Support both "message" and "edited_message"
        let msg = update
            .get("message")
            .or_else(|| update.get("edited_message"));
        let msg = match msg {
            Some(m) => m,
            None => continue,
        };

        let update_id = match update["update_id"].as_i64() {
            Some(id) => id,
            None => continue,
        };

        let chat_id = match msg["chat"]["id"].as_i64() {
            Some(id) => id,
            None => continue,
        };

        // Prefer a document image (original resolution) over a compressed photo.
        // Telegram compresses photo messages to ~1280px; documents preserve the original.
        let doc = &msg["document"];
        let doc_image_file_id = doc["mime_type"].as_str()
            .filter(|mime| mime.starts_with("image/"))
            .and_then(|_| doc["file_id"].as_str().map(|s| s.to_string()));

        // Fall back to the largest compressed photo if no document image is present.
        let photo_file_id = doc_image_file_id.or_else(|| {
            msg["photo"].as_array().and_then(|photos| {
                photos.last().and_then(|p| p["file_id"].as_str().map(|s| s.to_string()))
            })
        });

        let text = match msg["text"].as_str() {
            Some(t) => t.to_string(),
            None if photo_file_id.is_some() => {
                // Use caption as the prompt, empty string if absent
                msg["caption"].as_str().unwrap_or("").to_string()
            }
            None => continue, // Skip stickers, voice notes, etc.
        };

        results.push(TelegramUpdate {
            update_id,
            chat_id,
            text,
            photo_file_id,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_updates_returns_empty_on_empty_result() {
        let json = r#"{"ok":true,"result":[]}"#;
        assert!(parse_updates(json).is_empty());
    }

    #[test]
    fn parse_updates_extracts_update_id_chat_id_and_text() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 100,
                "message": {
                    "chat": {"id": 42},
                    "text": "hola"
                }
            }]
        }"#;
        let updates = parse_updates(json);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].update_id, 100);
        assert_eq!(updates[0].chat_id, 42);
        assert_eq!(updates[0].text, "hola");
    }

    #[test]
    fn parse_updates_skips_updates_without_text() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 101,
                "message": {
                    "chat": {"id": 5},
                    "sticker": {}
                }
            }]
        }"#;
        assert!(parse_updates(json).is_empty());
    }

    #[test]
    fn parse_updates_handles_edited_message() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 200,
                "edited_message": {
                    "chat": {"id": 99},
                    "text": "corregido"
                }
            }]
        }"#;
        let updates = parse_updates(json);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].text, "corregido");
    }

    #[test]
    fn parse_updates_returns_empty_on_malformed_json() {
        assert!(parse_updates("not json at all").is_empty());
    }

    #[test]
    fn parse_updates_extracts_photo_file_id() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 300,
                "message": {
                    "chat": {"id": 7},
                    "caption": "qué hay aquí?",
                    "photo": [
                        {"file_id": "small_id", "width": 90,  "height": 90},
                        {"file_id": "large_id", "width": 800, "height": 600}
                    ]
                }
            }]
        }"#;
        let updates = parse_updates(json);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].photo_file_id.as_deref(), Some("large_id"));
        assert_eq!(updates[0].text, "qué hay aquí?");
    }

    #[test]
    fn parse_updates_prefers_document_image_over_compressed_photo() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 301,
                "message": {
                    "chat": {"id": 8},
                    "caption": "analiza esto",
                    "document": {
                        "file_id": "doc_original_id",
                        "file_name": "photo.jpg",
                        "mime_type": "image/jpeg"
                    }
                }
            }]
        }"#;
        let updates = parse_updates(json);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].photo_file_id.as_deref(), Some("doc_original_id"));
        assert_eq!(updates[0].text, "analiza esto");
    }

    #[test]
    fn parse_updates_ignores_non_image_documents() {
        let json = r#"{
            "ok": true,
            "result": [{
                "update_id": 302,
                "message": {
                    "chat": {"id": 9},
                    "document": {
                        "file_id": "pdf_id",
                        "file_name": "report.pdf",
                        "mime_type": "application/pdf"
                    }
                }
            }]
        }"#;
        let updates = parse_updates(json);
        assert!(updates.is_empty(), "non-image document should be skipped");
    }

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
        )
        .unwrap();

        let report = read_usage_report(path);

        assert!(report.contains("2 ordenes"), "got: {report}");
        assert!(report.contains("0.0100"), "total cost; got: {report}");
        assert!(report.contains("0.0050"), "avg cost; got: {report}");
        assert!(report.contains("1980"), "total tokens; got: {report}");
        assert!(report.contains("adios"), "most expensive query; got: {report}");

        std::fs::remove_file(path).ok();
    }
}
