use std::env;

use voice_assistant::domain::model::{Language, WakeWord};
use voice_assistant::domain::service::VoiceListenerService;
use voice_assistant::infrastructure::audio::MicrophoneCapturer;
use voice_assistant::infrastructure::claude_handler::ClaudeCodeHandler;
use voice_assistant::infrastructure::speaker::GTTSSpeaker;
use voice_assistant::infrastructure::transcriber::GoogleTranscriber;

fn main() {
    let wake_word_str = env::var("WAKE_WORD").unwrap_or_else(|_| "claudito".into());
    let lang_code     = env::var("VOICE_LANGUAGE").unwrap_or_else(|_| "es-ES".into());

    let wake_word  = WakeWord::new(wake_word_str).expect("invalid WAKE_WORD");
    let language   = Language::new(lang_code).expect("invalid VOICE_LANGUAGE");

    let capturer      = Box::new(MicrophoneCapturer::new());
    let transcriber   = GoogleTranscriber::new();
    let order_handler = std::sync::Arc::new(ClaudeCodeHandler::new());
    let speaker       = GTTSSpeaker::new();

    let mut service = VoiceListenerService::new(
        capturer,
        transcriber,
        order_handler,
        speaker,
        wake_word,
        language,
    );

    service.run();
}
