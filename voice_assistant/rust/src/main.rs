use std::env;

use voice_assistant::cli::{parse_args, CliArgs};
use voice_assistant::domain::model::{Language, WakeWord};
use voice_assistant::domain::ports::OrderHandler;
use voice_assistant::domain::service::VoiceListenerService;
use voice_assistant::infrastructure::audio::MicrophoneCapturer;
use voice_assistant::infrastructure::claude_handler::ClaudeCodeHandler;
use voice_assistant::infrastructure::speaker::GTTSSpeaker;
use voice_assistant::infrastructure::telegram_bot::TelegramBot;
use voice_assistant::infrastructure::transcriber::GoogleTranscriber;

fn run_listen_mode(order_handler: std::sync::Arc<ClaudeCodeHandler>) {
    let wake_word_str = env::var("WAKE_WORD").unwrap_or_else(|_| "claudito".into());
    let lang_code     = env::var("VOICE_LANGUAGE").unwrap_or_else(|_| "es-ES".into());

    let wake_word = WakeWord::new(wake_word_str).expect("invalid WAKE_WORD");
    let language  = Language::new(lang_code).expect("invalid VOICE_LANGUAGE");

    let capturer    = Box::new(MicrophoneCapturer::new());
    let transcriber = GoogleTranscriber::new();
    let speaker     = GTTSSpeaker::new();

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

fn run_telegram_mode() {
    let token = std::env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN must be set for --telegram mode");
    TelegramBot::new(token).run(|| std::sync::Arc::new(ClaudeCodeHandler::new()));
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match parse_args(&args) {
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        Ok(CliArgs::BothMode) => {
            let order_handler = std::sync::Arc::new(ClaudeCodeHandler::new());
            std::thread::spawn(move || run_listen_mode(order_handler));
            run_telegram_mode();
        }
        Ok(CliArgs::TelegramMode) => {
            run_telegram_mode();
        }
        Ok(cli_args) => {
            let order_handler = std::sync::Arc::new(ClaudeCodeHandler::new());
            match cli_args {
                CliArgs::DirectOrder(order) => {
                    println!("Order: {order:?}");
                    let response = order_handler.handle(&order);
                    println!("Claudito: {response}");
                }
                CliArgs::ListenMode => {
                    run_listen_mode(order_handler);
                }
                CliArgs::TelegramMode | CliArgs::BothMode => unreachable!(),
            }
        }
    }
}
