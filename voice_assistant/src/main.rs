use std::env;

use voice_assistant::container;

fn main() {
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN must be set");
    container::build_telegram_bot(token).run(container::make_order_handler);
}
