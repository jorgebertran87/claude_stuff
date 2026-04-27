pub trait OrderHandler: Send + Sync {
    fn handle(&self, order: &str) -> String;
    fn reset_session(&self);
}

/// Port for accessing Google Sheets data and managing OAuth credentials.
pub trait GoogleSheetsGateway: Send + Sync {
    fn auth_url(&self) -> Option<String>;
    fn exchange_code(&self, code: &str) -> Result<(), String>;
    fn fetch_as_text(&self) -> Result<String, String>;
}

/// Port for synthesizing text to MP3 audio bytes.
pub trait TextSynthesizer: Send + Sync {
    fn synthesize_text(&self, text: &str) -> Vec<u8>;
    fn synthesize_alexa_spotify(&self, text: &str) -> Vec<u8>;
}

/// Port for analyzing images using an AI model.
pub trait ImageAnalyzer: Send + Sync {
    fn analyze(&self, bytes: &[u8], caption: &str, model: &str) -> String;
}
