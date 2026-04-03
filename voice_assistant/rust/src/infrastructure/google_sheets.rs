use serde_json::Value;

const TOKEN_FILE: &str = ".google_refresh_token";

pub struct SheetsClient {
    spreadsheet_id: String,
    client_id:      String,
    client_secret:  String,
    refresh_token:  String,
}

impl SheetsClient {
    /// Build from environment variables.
    /// `GOOGLE_REFRESH_TOKEN` env var takes precedence; falls back to `.google_refresh_token` file.
    /// Returns `None` if any required value is missing.
    pub fn from_env() -> Option<Self> {
        let refresh_token = std::env::var("GOOGLE_REFRESH_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::fs::read_to_string(TOKEN_FILE)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })?;

        Some(Self {
            spreadsheet_id: std::env::var("GOOGLE_SPREADSHEET_ID").ok()?,
            client_id:      std::env::var("GOOGLE_CLIENT_ID").ok()?,
            client_secret:  std::env::var("GOOGLE_CLIENT_SECRET").ok()?,
            refresh_token,
        })
    }

    /// Exchange the refresh token for a short-lived access token.
    fn access_token(&self) -> Result<String, String> {
        let body = format!(
            "client_id={}&client_secret={}&refresh_token={}&grant_type=refresh_token",
            self.client_id, self.client_secret, self.refresh_token
        );
        let resp = ureq::post("https://oauth2.googleapis.com/token")
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(&body)
            .map_err(|e| format!("token request: {e}"))?;
        let body = resp.into_string().map_err(|e| format!("token read: {e}"))?;
        let json: Value = serde_json::from_str(&body).map_err(|e| format!("token parse: {e}"))?;
        json["access_token"]
            .as_str()
            .ok_or_else(|| format!("no access_token in response: {json}"))
            .map(|s| s.to_string())
    }

    /// Fetch all cell values from the spreadsheet and return them as
    /// tab-separated rows (one row per line), ready to send to Claude.
    pub fn fetch_as_text(&self) -> Result<String, String> {
        let token = self.access_token()?;
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/A1:Z1000",
            self.spreadsheet_id
        );
        let resp = ureq::get(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .call()
            .map_err(|e| format!("sheets request: {e}"))?;
        let body = resp.into_string().map_err(|e| format!("sheets read: {e}"))?;
        let json: Value = serde_json::from_str(&body).map_err(|e| format!("sheets parse: {e}"))?;

        let rows = json["values"]
            .as_array()
            .ok_or_else(|| "no 'values' in Sheets response".to_string())?;

        let text = rows
            .iter()
            .map(|row| {
                row.as_array()
                    .map(|cells| {
                        cells
                            .iter()
                            .map(|c| c.as_str().unwrap_or(""))
                            .collect::<Vec<_>>()
                            .join("\t")
                    })
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }
}

/// Exchange a one-time authorization code for a refresh token and persist it to disk.
pub fn exchange_and_save_token(code: &str) -> Result<(), String> {
    let client_id     = std::env::var("GOOGLE_CLIENT_ID")    .map_err(|_| "GOOGLE_CLIENT_ID not set".to_string())?;
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| "GOOGLE_CLIENT_SECRET not set".to_string())?;

    let body = format!(
        "client_id={}&client_secret={}&code={}&grant_type=authorization_code\
         &redirect_uri=urn:ietf:wg:oauth:2.0:oob",
        client_id, client_secret, code
    );
    let resp = ureq::post("https://oauth2.googleapis.com/token")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
        .map_err(|e| format!("token request: {e}"))?;
    let body = resp.into_string().map_err(|e| format!("token read: {e}"))?;
    let json: Value = serde_json::from_str(&body).map_err(|e| format!("token parse: {e}"))?;
    let token = json["refresh_token"]
        .as_str()
        .ok_or_else(|| format!("no refresh_token in response: {json}"))?;

    std::fs::write(TOKEN_FILE, token).map_err(|e| format!("write {TOKEN_FILE}: {e}"))
}

/// Build the Google OAuth2 authorization URL using GOOGLE_CLIENT_ID from env.
pub fn auth_url() -> Option<String> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty())?;
    Some(format!(
        "https://accounts.google.com/o/oauth2/auth\
         ?client_id={client_id}\
         &redirect_uri=urn:ietf:wg:oauth:2.0:oob\
         &response_type=code\
         &scope=https://www.googleapis.com/auth/spreadsheets.readonly"
    ))
}
