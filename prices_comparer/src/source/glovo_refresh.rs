use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::token_store::TokenStore;

/// Glovo app version sent alongside the device header on the refresh call.
const APP_VERSION: &str = "v1.2329.0";

/// The long-lived credentials needed to mint fresh access tokens: the
/// rotating refresh token and the device urn the refresh endpoint binds to.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RefreshCreds {
    pub refresh_token: String,
    pub device_urn: String,
}

/// Persists the Glovo refresh credentials to a JSON file under `/data`.
/// The refresh token rotates on every refresh, so it is written back here.
pub struct RefreshStore {
    path: PathBuf,
}

impl RefreshStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The current credentials, or `None` when the file is missing or holds
    /// no refresh token.
    pub fn current(&self) -> Option<RefreshCreds> {
        let raw = std::fs::read_to_string(&self.path).ok()?;
        let creds: RefreshCreds = serde_json::from_str(&raw).ok()?;
        if creds.refresh_token.trim().is_empty() {
            None
        } else {
            Some(creds)
        }
    }

    /// Persist credentials, written atomically (temp file + rename).
    pub fn save(&self, creds: &RefreshCreds) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string(creds)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Why a refresh did not produce a new token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshError {
    /// No refresh token is configured yet — nothing to refresh.
    NotConfigured,
    /// The refresh token was rejected (expired/revoked); re-seed is needed.
    Rejected,
    /// The endpoint could not be reached or returned unusable data.
    Unavailable,
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshError::NotConfigured => write!(f, "no refresh token configured"),
            RefreshError::Rejected => write!(f, "refresh token rejected"),
            RefreshError::Unavailable => write!(f, "refresh endpoint unavailable"),
        }
    }
}

/// Exchanges the stored refresh token for a fresh 20-minute access token via
/// Glovo's `/oauth/refresh`, writing the new access token where the bot reads
/// it and persisting the rotated refresh token for the next cycle.
pub struct GlovoRefresher {
    client: reqwest::Client,
    base_url: String,
    tokens: TokenStore,
    creds: RefreshStore,
}

#[derive(Deserialize)]
struct RefreshResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
}

impl GlovoRefresher {
    /// `base_url` is `https://api.glovoapp.com` in production or a mock in tests.
    pub fn new(base_url: String, tokens: TokenStore, creds: RefreshStore) -> Self {
        Self { client: reqwest::Client::new(), base_url, tokens, creds }
    }

    pub async fn refresh(&self) -> Result<(), RefreshError> {
        let creds = self.creds.current().ok_or(RefreshError::NotConfigured)?;

        let response = self
            .client
            .post(format!("{}/oauth/refresh", self.base_url))
            .header("accept", "application/json, text/plain, */*")
            .header("glovo-device-urn", &creds.device_urn)
            .header("glovo-api-version", "14")
            .header("glovo-app-platform", "web")
            .header("glovo-app-type", "customer")
            .header("glovo-app-version", APP_VERSION)
            .json(&serde_json::json!({ "refreshToken": creds.refresh_token }))
            .send()
            .await
            .map_err(|_| RefreshError::Unavailable)?;

        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(RefreshError::Rejected);
        }
        let response = response.error_for_status().map_err(|_| RefreshError::Unavailable)?;
        let body: RefreshResponse = response.json().await.map_err(|_| RefreshError::Unavailable)?;

        self.tokens.set(&body.access_token).map_err(|_| RefreshError::Unavailable)?;
        self.creds
            .save(&RefreshCreds {
                refresh_token: body.refresh_token,
                device_urn: creds.device_urn,
            })
            .map_err(|_| RefreshError::Unavailable)?;
        Ok(())
    }
}
