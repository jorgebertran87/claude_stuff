use prices_comparer::basket::{BasketSource, OrderNormalizer};
use prices_comparer::comparer::StoreSource;
use prices_comparer::normalizer::ClaudeCliNormalizer;
use prices_comparer::source::{
    dia::DiaSource, glovo::GlovoSource, lidl::LidlSource, mercadona::MercadonaSource,
};
use prices_comparer::telegram::TelegramBot;
use prices_comparer::token_store::TokenStore;

fn env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|_| anyhow::anyhow!("{name} must be set"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bot_token = env("TELEGRAM_BOT_TOKEN")?;
    let chat_id: i64 = env("TELEGRAM_CHAT_ID")?
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("TELEGRAM_CHAT_ID must be an integer"))?;

    let flare_url = std::env::var("FLARESOLVERR_URL")
        .unwrap_or_else(|_| "http://flaresolverr:8191".to_string());
    // The Algolia app id is public (it is in the shop's page source); the
    // search-only API key still has to be supplied explicitly.
    let mercadona_app_id = std::env::var("MERCADONA_APP_ID")
        .unwrap_or_else(|_| "7UZJKL1DJ0".to_string());
    let mercadona_api_key = env("MERCADONA_API_KEY")?;

    let stores: Vec<Box<dyn StoreSource>> = vec![
        Box::new(MercadonaSource::new(
            "https://7uzjkl1dj0-dsn.algolia.net".to_string(),
            mercadona_app_id,
            mercadona_api_key,
        )),
        Box::new(DiaSource::new(flare_url)),
        Box::new(LidlSource::new("https://www.lidl.es".to_string())),
    ];

    // Glovo's token lives in a file the bot shares with the mitmproxy
    // capturer, so a freshly captured token takes effect without a restart.
    // A GLOVO_TOKEN in the environment seeds the file on first boot.
    let token_path = std::env::var("GLOVO_TOKEN_FILE")
        .unwrap_or_else(|_| "/data/glovo_token".to_string());
    let glovo_tokens = TokenStore::new(token_path.into());
    if let Ok(token) = std::env::var("GLOVO_TOKEN") {
        if !token.trim().is_empty() && glovo_tokens.current().is_none() {
            glovo_tokens.set(&token)?;
        }
    }
    let baskets: Vec<Box<dyn BasketSource>> = vec![Box::new(GlovoSource::new(
        "https://api.glovoapp.com".to_string(),
        glovo_tokens,
    ))];

    // Glovo orders are normalized through Claude before comparison; the bot
    // falls back to raw item names if it is unavailable.
    let normalizer: Box<dyn OrderNormalizer> = Box::new(ClaudeCliNormalizer::new());

    TelegramBot::new(
        "https://api.telegram.org".to_string(),
        bot_token,
        chat_id,
        stores,
        baskets,
        normalizer,
    )
    .run()
    .await;
    Ok(())
}
