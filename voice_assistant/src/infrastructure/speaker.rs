use std::io::Cursor;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, LazyLock, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use regex::Regex;
use whichlang::detect_language;

use shaku::Component;

use crate::domain::model::Language;
use crate::domain::ports::{AudioSpeaker, EchoRef, TextSynthesizer};
use crate::infrastructure::tts::{tts_segment, tts_chunks};

static RE_LINK:    LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap());
static RE_URL:     LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://\S+").unwrap());
static RE_BOLD:    LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*+([^*]*)\*+").unwrap());
static RE_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^#+\s+").unwrap());
static RE_BULLET:  LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[-*]\s+").unwrap());
static RE_CODE:    LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`[^`]*`").unwrap());
static RE_QUOTE:   LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""([^"]+)"|'([^']+)'"#).unwrap());

pub fn strip_markdown(text: &str) -> String {
    let s = RE_LINK.replace_all(text, "$1");
    let s = RE_URL.replace_all(&s, "");
    let s = RE_BOLD.replace_all(&s, "$1");
    let s = RE_HEADING.replace_all(&s, "");
    let s = RE_BULLET.replace_all(&s, "");
    let s = RE_CODE.replace_all(&s, "");
    s.trim().to_string()
}

pub fn detect_lang(text: &str) -> String {
    use whichlang::Lang;
    match detect_language(text) {
        Lang::Eng => "en",
        Lang::Spa => "es",
        Lang::Fra => "fr",
        Lang::Deu => "de",
        Lang::Por => "pt",
        Lang::Ita => "it",
        Lang::Nld => "nl",
        Lang::Rus => "ru",
        Lang::Swe => "sv",
        Lang::Tur => "tr",
        Lang::Kor => "ko",
        Lang::Ara => "ar",
        Lang::Hin => "hi",
        Lang::Cmn => "zh-CN",
        Lang::Jpn => "ja",
        _ => "en",
    }
    .to_string()
}

pub fn alexa_spotify_title(text: &str) -> Option<(String, String)> {
    let lower = text.to_lowercase();
    if !lower.contains("alexa") || !lower.contains("spotify") {
        return None;
    }
    let m = RE_QUOTE.captures(text)?;
    let title = m.get(1).or_else(|| m.get(2))?.as_str().to_string();
    let lang = if detect_lang(&title) == "es" { "es".into() } else { "en".into() };
    Some((title, lang))
}

pub fn build_alexa_command(title: &str, lang: &str) -> String {
    match lang {
        "es" => format!("Alexa, pon {} en Spotify", title),
        _    => format!("Alexa, play {} on Spotify", title),
    }
}

pub fn synthesize_text(text: &str) -> Vec<u8> {
    let clean = strip_markdown(text);
    if clean.trim().is_empty() {
        return Vec::new();
    }
    let lang = detect_lang(&clean);
    let mut all_bytes: Vec<u8> = Vec::new();
    for chunk in tts_chunks(&clean) {
        match tts_segment(&chunk, &lang) {
            Ok(seg) => all_bytes.extend_from_slice(seg.raw_data()),
            Err(e) => eprintln!("[tts error: {e}]"),
        }
    }
    if all_bytes.is_empty() {
        return Vec::new();
    }
    apply_atempo(all_bytes, 1.2)
}

pub fn synthesize_alexa_spotify(text: &str) -> Vec<u8> {
    let unified = alexa_spotify_title(text)
        .map(|(title, lang)| build_alexa_command(&title, &lang))
        .unwrap_or_else(|| strip_markdown(text));
    synthesize_text(&unified)
}

pub fn disconnect_bt_speaker() {
    let mac = match std::env::var("BT_SPEAKER_MAC") {
        Ok(m) if !m.is_empty() => m,
        _ => {
            eprintln!("[bt: BT_SPEAKER_MAC not set, skipping disconnect]");
            return;
        }
    };
    eprintln!("[bt: disconnecting {mac} after inactivity]");
    let _ = Command::new("bluetoothctl")
        .args(["disconnect", &mac])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn apply_atempo(bytes: Vec<u8>, speed: f32) -> Vec<u8> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let input_path  = format!("/tmp/tts_atempo_in_{nanos}.mp3");
    let output_path = format!("/tmp/tts_atempo_out_{nanos}.mp3");
    if std::fs::write(&input_path, &bytes).is_err() {
        return bytes;
    }
    let ok = Command::new("ffmpeg")
        .args([
            "-y", "-loglevel", "quiet",
            "-i", &input_path,
            "-af", &format!("atempo={speed}"),
            "-f", "mp3", &output_path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&input_path);
    if ok {
        let result = std::fs::read(&output_path).unwrap_or(bytes);
        let _ = std::fs::remove_file(&output_path);
        result
    } else {
        bytes
    }
}

#[derive(Component)]
#[shaku(interface = TextSynthesizer)]
pub struct PiperTextSynthesizer;

impl TextSynthesizer for PiperTextSynthesizer {
    fn synthesize_text(&self, text: &str) -> Vec<u8> {
        synthesize_text(text)
    }

    fn synthesize_alexa_spotify(&self, text: &str) -> Vec<u8> {
        synthesize_alexa_spotify(text)
    }
}

// ── GttsTextSynthesizer (Google Translate TTS, online) ───────────────────────

#[derive(Component)]
#[shaku(interface = TextSynthesizer)]
pub struct GttsTextSynthesizer;

impl TextSynthesizer for GttsTextSynthesizer {
    fn synthesize_text(&self, text: &str) -> Vec<u8> {
        synthesize_text(text)
    }

    fn synthesize_alexa_spotify(&self, text: &str) -> Vec<u8> {
        synthesize_alexa_spotify(text)
    }
}

// ── PiperSpeaker ──────────────────────────────────────────────────────────────

#[derive(Component)]
#[shaku(interface = AudioSpeaker)]
pub struct PiperSpeaker {
    #[shaku(default)]
    stop_signal: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl PiperSpeaker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { stop_signal: Arc::new(Mutex::new(None)) })
    }

    fn play_bytes(&self, bytes: &[u8], on_start: Option<Box<dyn FnOnce() + Send>>) {
        if bytes.is_empty() {
            return;
        }
        if let Some(cb) = on_start {
            cb();
        }

        let stop = Arc::new(AtomicBool::new(false));
        *self.stop_signal.lock().unwrap() = Some(Arc::clone(&stop));
        let owned = bytes.to_vec();

        thread::spawn(move || {
            match rodio::Decoder::new(Cursor::new(owned)) {
                Ok(source) => {
                    if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                        if let Ok(sink) = rodio::Sink::try_new(&handle) {
                            sink.append(source);
                            while !sink.empty() && !stop.load(Ordering::SeqCst) {
                                thread::sleep(Duration::from_millis(50));
                            }
                            if stop.load(Ordering::SeqCst) {
                                sink.stop();
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[playback: decode error: {e}]"),
            }
        });
    }
}

impl AudioSpeaker for PiperSpeaker {
    fn speak(&self, text: &str, language: &Language, on_playback_start: Option<Box<dyn FnOnce() + Send>>) {
        let (unified, lang) = match alexa_spotify_title(text) {
            Some((title, ref tl)) => (build_alexa_command(&title, tl), tl.clone()),
            None => (strip_markdown(text), language.lang_prefix().to_string()),
        };

        let mut all_bytes: Vec<u8> = Vec::new();
        for piece in tts_chunks(&unified) {
            match tts_segment(&piece, &lang) {
                Ok(seg) => all_bytes.extend_from_slice(seg.raw_data()),
                Err(e)  => eprintln!("TTS error: {e}"),
            }
        }

        if !all_bytes.is_empty() {
            self.play_bytes(&all_bytes, on_playback_start);
        }
    }

    fn stop(&self) {
        if let Some(stop) = self.stop_signal.lock().unwrap().take() {
            stop.store(true, Ordering::SeqCst);
        }
    }

    fn beep(&self) {
        if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
            if let Ok(sink) = rodio::Sink::try_new(&handle) {
                use rodio::Source;
                let source = rodio::source::SineWave::new(440.0)
                    .take_duration(Duration::from_millis(200))
                    .amplify(0.5);
                sink.append(source);
                sink.sleep_until_end();
            }
        }
    }

    fn play_melody(&self, stop_signal: Arc<AtomicBool>) {
        while !stop_signal.load(Ordering::SeqCst) {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                if let Ok(sink) = rodio::Sink::try_new(&handle) {
                    use rodio::Source;
                    let source = rodio::source::SineWave::new(520.0)
                        .take_duration(Duration::from_millis(400))
                        .amplify(0.3);
                    sink.append(source);
                    // Check stop during play
                    while !sink.empty() && !stop_signal.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(50));
                    }
                    if stop_signal.load(Ordering::SeqCst) {
                        sink.stop();
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
    }

    fn get_echo_reference(&self) -> Option<EchoRef> {
        None
    }

    fn disconnect(&self) {
        disconnect_bt_speaker();
    }
}

// ── GTTSSpeaker (ffplay-based, pre-rodio) ─────────────────────────────────────

#[derive(Component)]
#[shaku(interface = AudioSpeaker)]
pub struct GTTSSpeaker {
    #[shaku(default)]
    current_pid: Arc<Mutex<Option<u32>>>,
}

impl GTTSSpeaker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { current_pid: Arc::new(Mutex::new(None)) })
    }

    fn play_bytes(&self, bytes: &[u8], on_start: Option<Box<dyn FnOnce() + Send>>) {
        let tmp = "/tmp/voice_response.mp3";
        let _ = std::fs::write(tmp, bytes);

        if let Some(cb) = on_start {
            cb();
        }

        if let Ok(mut child) = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet",
                   "-af", "atempo=1.2",
                   tmp])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            *self.current_pid.lock().unwrap() = Some(child.id());
            let _ = child.wait();
            *self.current_pid.lock().unwrap() = None;
        }
    }
}

impl AudioSpeaker for GTTSSpeaker {
    fn speak(&self, text: &str, language: &Language, on_playback_start: Option<Box<dyn FnOnce() + Send>>) {
        let (unified, lang) = match alexa_spotify_title(text) {
            Some((title, ref tl)) => (build_alexa_command(&title, tl), tl.clone()),
            None => (strip_markdown(text), language.lang_prefix().to_string()),
        };

        let mut all_bytes: Vec<u8> = Vec::new();
        for piece in tts_chunks(&unified) {
            match tts_segment(&piece, &lang) {
                Ok(seg) => all_bytes.extend_from_slice(seg.raw_data()),
                Err(e)  => eprintln!("TTS error: {e}"),
            }
        }

        if !all_bytes.is_empty() {
            self.play_bytes(&all_bytes, on_playback_start);
        }
    }

    fn stop(&self) {
        if let Some(pid) = *self.current_pid.lock().unwrap() {
            let _ = Command::new("kill")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

    fn beep(&self) {
        let _ = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet",
                   "-f", "lavfi", "-i", "sine=frequency=440:duration=0.2"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    fn play_melody(&self, stop_signal: Arc<AtomicBool>) {
        while !stop_signal.load(Ordering::SeqCst) {
            let _ = Command::new("ffplay")
                .args(["-nodisp", "-autoexit", "-loglevel", "quiet",
                       "-f", "lavfi", "-i", "sine=frequency=520:duration=0.4"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            thread::sleep(Duration::from_millis(200));
        }
    }

    fn get_echo_reference(&self) -> Option<EchoRef> {
        None
    }

    fn disconnect(&self) {
        disconnect_bt_speaker();
    }
}
