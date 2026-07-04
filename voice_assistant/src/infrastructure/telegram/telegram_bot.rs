use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::domain::ports::{
    AudioPlayer, MinesweeperAnalyzer, OrderHandler,
    SkillCommands, TextSynthesizer,
};

// Re-export gateway types so external code (tests, container) keeps its import path.
pub use super::telegram_gateway::{
    TelegramGateway, TelegramUpdate, UreqGateway, UreqGatewayParameters,
};

const MODEL_HAIKU:  &str = "claude-haiku-4-5-20251001";
const MODEL_SONNET: &str = "claude-sonnet-4-6";
const MODEL_OPUS:   &str = "claude-opus-4-6";

fn resolve_model(name: &str) -> Option<&'static str> {
    match name.trim().to_lowercase().as_str() {
        "haiku"  => Some(MODEL_HAIKU),
        "sonnet" => Some(MODEL_SONNET),
        "opus"   => Some(MODEL_OPUS),
        _ => None,
    }
}

pub struct TelegramBot {
    gateway: Arc<dyn TelegramGateway>,
    synthesizer: Arc<dyn TextSynthesizer>,
    minesweeper: Arc<dyn MinesweeperAnalyzer>,
    skills: Arc<dyn SkillCommands>,
    audio_player: Arc<dyn AudioPlayer>,
    allowed_chat_ids: Vec<i64>,
}

impl TelegramBot {
    #[allow(clippy::too_many_arguments)]
    pub fn with_injectable(
        gateway: Arc<dyn TelegramGateway>,
        synthesizer: Arc<dyn TextSynthesizer>,
        minesweeper: Arc<dyn MinesweeperAnalyzer>,
        skills: Arc<dyn SkillCommands>,
        audio_player: Arc<dyn AudioPlayer>,
        allowed_chat_ids: Vec<i64>,
    ) -> Self {
        Self {
            gateway, synthesizer,
            minesweeper, skills, audio_player, allowed_chat_ids,
        }
    }

    fn is_allowed(&self, chat_id: i64) -> bool {
        self.allowed_chat_ids.is_empty() || self.allowed_chat_ids.contains(&chat_id)
    }

    fn spawn_minesweeper_analysis(
        gateway: Arc<dyn TelegramGateway>,
        minesweeper: Arc<dyn MinesweeperAnalyzer>,
        chat_id: i64,
        file_id: String,
        caption: String,
        model: String,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            eprintln!("[minesweeper: chat={} image={} model={}]", chat_id, file_id, model);
            let bytes = match gateway.download_file(&file_id) {
                Some(b) => b,
                None => {
                    gateway.post_message(chat_id, "No se pudo descargar la imagen.");
                    return;
                }
            };
            gateway.post_message(chat_id, "Analizando tablero de buscaminas...");
            let json = match minesweeper.parse_board(&bytes) {
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
            let response = minesweeper.analyze(&json, &caption, &model);
            let msg = if response.trim().is_empty() {
                "No pude obtener una respuesta de Claude.".to_string()
            } else {
                response
            };
            gateway.post_message(chat_id, &msg);
        })
    }

    pub fn run_once(
        &self,
        make_handler: &dyn Fn() -> Arc<dyn OrderHandler>,
        handlers: &mut HashMap<i64, Arc<dyn OrderHandler>>,
        voice_mode_chats: &mut HashSet<i64>,
        pending_image_chats: &mut HashMap<i64, String>,
        current_model: &mut String,
        offset: &mut i64,
        speak_text: &dyn Fn(&str),
        on_voice: &dyn Fn(),
    ) -> Vec<thread::JoinHandle<()>> {
        let mut handles = Vec::new();
        let updates = self.gateway.fetch_updates(*offset);

        for update in updates {
            *offset = update.update_id + 1;

            if !self.is_allowed(update.chat_id) {
                eprintln!("[telegram: ignoring unauthorised chat {}]", update.chat_id);
                continue;
            }

            if let Some(ref file_id) = update.photo_file_id {
                let caption = update.text.trim();
                if caption.is_empty() {
                    pending_image_chats.insert(update.chat_id, file_id.clone());
                    self.gateway.post_message(update.chat_id, "¿Qué quieres que haga con esta imagen?");
                } else {
                    let lower = caption.to_lowercase();
                    if lower.contains("buscaminas") || lower.contains("minesweeper") {
                        let handle = Self::spawn_minesweeper_analysis(
                            Arc::clone(&self.gateway), Arc::clone(&self.minesweeper), update.chat_id,
                            file_id.clone(), caption.to_string(), current_model.clone(),
                        );
                        handles.push(handle);
                    } else {
                        self.gateway.post_message(
                            update.chat_id,
                            "El análisis de imágenes no está disponible con DeepSeek.",
                        );
                    }
                }
                continue;
            }

            let text = update.text.trim();

            if let Some(file_id) = pending_image_chats.remove(&update.chat_id) {
                if !text.starts_with('/') {
                    let lower_text = text.to_lowercase();
                    if lower_text.contains("buscaminas") || lower_text.contains("minesweeper") {
                        let handle = Self::spawn_minesweeper_analysis(
                            Arc::clone(&self.gateway), Arc::clone(&self.minesweeper), update.chat_id,
                            file_id, text.to_string(), current_model.clone(),
                        );
                        handles.push(handle);
                    } else {
                        self.gateway.post_message(
                            update.chat_id,
                            "El análisis de imágenes no está disponible con DeepSeek.",
                        );
                    }
                    continue;
                }
                pending_image_chats.insert(update.chat_id, file_id);
            }

            if text == "/list" {
                self.gateway.post_message(update.chat_id, &format!("\
/list         — muestra este mensaje\n\
/reset        — reinicia la sesión de Claude\n\
/usage        — resumen de tokens y coste acumulado\n\
/voice_mode   — activa/desactiva respuestas por voz\n\
/volume [+N|-N|N] — consulta o ajusta el volumen del altavoz\n\
/model [haiku|sonnet|opus] — cambia el modelo (actual: {current_model})\n\
/bus          — próximas salidas hacia Alameda Principal\n\
/connect_speakers — conecta el altavoz Bluetooth"));
                continue;
            }

            if text == "/reset" {
                if let Some(handler) = handlers.get(&update.chat_id) {
                    handler.reset_session();
                }
                self.gateway.post_message(update.chat_id, "Sesión reiniciada.");
                continue;
            }

            if text == "/usage" {
                let report = self.skills.usage_report(".orders_tokens");
                self.gateway.post_message(update.chat_id, &report);
                continue;
            }

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

            if text.starts_with("/model") {
                let arg = text["/model".len()..].trim();
                let msg = if arg.is_empty() {
                    format!("Modelo actual: {current_model}")
                } else {
                    match resolve_model(arg) {
                        Some(m) => { *current_model = m.to_string(); format!("Modelo cambiado a: {m}") }
                        None => format!("Modelo desconocido: '{arg}'. Usa haiku, sonnet u opus."),
                    }
                };
                self.gateway.post_message(update.chat_id, &msg);
                continue;
            }

            if text.starts_with("/bus") {
                let stop_code = text["/bus".len()..].trim().to_string();
                let gateway   = Arc::clone(&self.gateway);
                let skills    = Arc::clone(&self.skills);
                let model     = current_model.clone();
                let chat_id   = update.chat_id;
                handles.push(thread::spawn(move || {
                    let msg = skills.bus(&model, &stop_code);
                    gateway.post_message(chat_id, &msg);
                }));
                continue;
            }

            if text == "/connect_speakers" {
                let msg = self.skills.connect_speakers();
                self.gateway.post_message(update.chat_id, &msg);
                continue;
            }

            if text.starts_with("/volume") {
                let arg = text["/volume".len()..].trim();
                let msg = self.skills.volume(arg);
                self.gateway.post_message(update.chat_id, &msg);
                continue;
            }

            if text.starts_with('/') {
                continue;
            }

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
                    self.audio_player.play(&bytes);
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

    pub fn run(&self, make_handler: impl Fn() -> Arc<dyn OrderHandler>) {
        use std::sync::Mutex;

        eprintln!("[telegram bot starting, allowed chats: {:?}]", self.allowed_chat_ids);
        let mut offset: i64 = 0;
        let mut handlers: HashMap<i64, Arc<dyn OrderHandler>> = HashMap::new();
        let mut voice_mode_chats: HashSet<i64> = HashSet::new();
        let mut pending_image_chats: HashMap<i64, String> = HashMap::new();
        let mut current_model: String = MODEL_HAIKU.to_string();

        let last_voice: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let last_voice_bg = Arc::clone(&last_voice);

        let audio_player_bg = Arc::clone(&self.audio_player);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(30));
                let elapsed = last_voice_bg.lock().unwrap()
                    .map(|t| t.elapsed())
                    .unwrap_or(Duration::ZERO);
                if elapsed >= Duration::from_secs(300) {
                    audio_player_bg.disconnect();
                    *last_voice_bg.lock().unwrap() = None;
                }
            }
        });

        let synthesizer = Arc::clone(&self.synthesizer);
        let audio_player = Arc::clone(&self.audio_player);
        let speak_text = move |text: &str| {
            eprintln!("[voice_mode: synthesizing {} chars]", text.len());
            let bytes = synthesizer.synthesize_text(text);
            if bytes.is_empty() {
                eprintln!("[voice_mode: synthesis returned empty bytes, skipping playback]");
            } else {
                eprintln!("[voice_mode: playing {} bytes]", bytes.len());
                audio_player.play(&bytes);
            }
        };

        let on_voice = || {
            *last_voice.lock().unwrap() = Some(Instant::now());
        };

        loop {
            let prev_offset = offset;
            self.run_once(
                &make_handler, &mut handlers, &mut voice_mode_chats,
                &mut pending_image_chats,
                &mut current_model, &mut offset, &speak_text, &on_voice,
            );
            // When no updates were processed (fetch error or truly empty),
            // pause briefly to avoid hammering Telegram on 409 / timeout.
            if offset == prev_offset {
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }
}
