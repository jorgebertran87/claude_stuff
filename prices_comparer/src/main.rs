use std::sync::Arc;

use prices_comparer::basket::{BasketSource, OrderNormalizer};
use prices_comparer::comparer::{ProductSelector, StoreSource};
use prices_comparer::normalizer::{DeepSeekNormalizer, DeepSeekProductSelector};
use prices_comparer::source::glovo_refresh::{GlovoRefresher, RefreshCreds, RefreshStore};
use prices_comparer::source::{dia::DiaSource, glovo::GlovoSource, mercadona::MercadonaSource};
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

    // DeepSeek powers both the order normalizer and the per-store product
    // selector; one client shared across the stores. DEEPSEEK_MODEL picks the
    // model (see DeepSeek's docs for current ids).
    let deepseek_key = env("DEEPSEEK_API_KEY")?;
    let deepseek_model =
        std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());
    let selector: Arc<dyn ProductSelector> =
        Arc::new(DeepSeekProductSelector::new(deepseek_key.clone(), deepseek_model.clone()));

    let stores: Vec<Box<dyn StoreSource>> = vec![
        Box::new(MercadonaSource::new(
            "https://7uzjkl1dj0-dsn.algolia.net".to_string(),
            mercadona_app_id,
            mercadona_api_key,
            Some(selector.clone()),
        )),
        Box::new(DiaSource::new(flare_url, Some(selector.clone()))),
    ];

    // Glovo's token lives in a file the bot shares with the mitmproxy
    // capturer, so a freshly captured token takes effect without a restart.
    // A GLOVO_TOKEN in the environment seeds the file on first boot.
    let token_path = std::env::var("GLOVO_TOKEN_FILE")
        .unwrap_or_else(|_| "/data/glovo_token".to_string());
    let glovo_tokens = TokenStore::new(token_path.clone().into());
    if let Ok(token) = std::env::var("GLOVO_TOKEN") {
        if !token.trim().is_empty() && glovo_tokens.current().is_none() {
            glovo_tokens.set(&token)?;
        }
    }
    let baskets: Vec<Box<dyn BasketSource>> = vec![Box::new(GlovoSource::new(
        "https://api.glovoapp.com".to_string(),
        glovo_tokens,
    ))];

    // Glovo access tokens expire after ~20 min. When a refresh token + device
    // urn are configured, refresh in the background so /glovo stays alive.
    let refresh_path = std::env::var("GLOVO_REFRESH_FILE")
        .unwrap_or_else(|_| "/data/glovo_refresh.json".to_string());
    let refresh_store = RefreshStore::new(refresh_path.into());
    if refresh_store.current().is_none() {
        if let Ok(refresh_token) = std::env::var("GLOVO_REFRESH_TOKEN") {
            if !refresh_token.trim().is_empty() {
                let device_urn = std::env::var("GLOVO_DEVICE_URN").unwrap_or_default();
                refresh_store.save(&RefreshCreds { refresh_token, device_urn })?;
            }
        }
    }
    if refresh_store.current().is_some() {
        let refresher = GlovoRefresher::new(
            "https://api.glovoapp.com".to_string(),
            TokenStore::new(token_path.into()),
            refresh_store,
        );
        tokio::spawn(async move {
            loop {
                match refresher.refresh().await {
                    Ok(()) => eprintln!("[glovo] access token refreshed"),
                    Err(e) => eprintln!("[glovo] refresh failed: {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(900)).await;
            }
        });
    }

    // Glovo orders are normalized through DeepSeek before comparison; the bot
    // falls back to raw item names if it is unavailable.
    let normalizer: Box<dyn OrderNormalizer> =
        Box::new(DeepSeekNormalizer::new(deepseek_key, deepseek_model));

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
