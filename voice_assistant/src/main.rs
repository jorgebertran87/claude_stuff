use std::env;
use std::sync::Arc;

use shaku::HasComponent;

use voice_assistant::cli::{parse_args, CliArgs};
use voice_assistant::container::build_module;
use voice_assistant::domain::model::{Language, WakeWord};
use voice_assistant::domain::ports::{
    AudioSpeaker, GoogleSheetsGateway, ImageAnalyzer, OrderHandler, TextSynthesizer, Transcriber,
};
use voice_assistant::domain::service::VoiceListenerService;
use voice_assistant::infrastructure::audio::MicrophoneCapturer;
use voice_assistant::infrastructure::claude_handler::ClaudeCodeHandler;
use voice_assistant::infrastructure::telegram_bot::{TelegramBot, TelegramGateway};

fn main() {
    let args: Vec<String> = env::args().collect();

    match parse_args(&args) {
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        Ok(CliArgs::TelegramMode) => {
            let token = env::var("TELEGRAM_BOT_TOKEN")
                .expect("TELEGRAM_BOT_TOKEN must be set for --telegram mode");
            let allowed: Vec<i64> = env::var("TELEGRAM_ALLOWED_CHAT_IDS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect();

            let module = build_module(token);

            let gateway:        Arc<dyn TelegramGateway>     = module.resolve();
            let sheets:         Arc<dyn GoogleSheetsGateway> = module.resolve();
            let synthesizer:    Arc<dyn TextSynthesizer>     = module.resolve();
            let image_analyzer: Arc<dyn ImageAnalyzer>       = module.resolve();

            let bot = TelegramBot::with_injectable(gateway, sheets, synthesizer, image_analyzer, allowed);
            bot.run(|| Arc::new(ClaudeCodeHandler::new()));
        }
        Ok(cli_args) => {
            let module = build_module(String::new());

            match cli_args {
                CliArgs::DirectOrder(order) => {
                    let handler: Arc<dyn OrderHandler> = module.resolve();
                    println!("Order: {order:?}");
                    let response = handler.handle(&order);
                    println!("Claudito: {response}");
                }
                CliArgs::ListenMode => {
                    let wake_word_str =
                        env::var("WAKE_WORD").unwrap_or_else(|_| "claudito".into());
                    let lang_code =
                        env::var("VOICE_LANGUAGE").unwrap_or_else(|_| "es-ES".into());

                    let wake_word = WakeWord::new(wake_word_str).expect("invalid WAKE_WORD");
                    let language  = Language::new(lang_code).expect("invalid VOICE_LANGUAGE");

                    let transcriber:   Arc<dyn Transcriber>  = module.resolve();
                    let order_handler: Arc<dyn OrderHandler> = module.resolve();
                    let speaker:       Arc<dyn AudioSpeaker> = module.resolve();
                    let capturer = Box::new(MicrophoneCapturer::new());

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
                CliArgs::TelegramMode => unreachable!(),
            }
        }
    }
}
