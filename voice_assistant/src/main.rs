use std::env;

use voice_assistant::cli::{parse_args, CliArgs};
use voice_assistant::container;
use voice_assistant::domain::model::{Language, WakeWord};

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
            container::build_telegram_bot(token).run(container::make_order_handler);
        }
        Ok(CliArgs::DirectOrder(order)) => {
            let handler = container::build_order_handler();
            println!("Order: {order:?}");
            println!("Claudito: {}", handler.handle(&order));
        }
        Ok(CliArgs::ListenMode) => {
            let wake_word = WakeWord::new(
                env::var("WAKE_WORD").unwrap_or_else(|_| "claudito".into()),
            )
            .expect("invalid WAKE_WORD");
            let language = Language::new(
                env::var("VOICE_LANGUAGE").unwrap_or_else(|_| "es-ES".into()),
            )
            .expect("invalid VOICE_LANGUAGE");

            container::build_voice_service(wake_word, language).run();
        }
    }
}
