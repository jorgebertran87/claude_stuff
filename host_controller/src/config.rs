//! Configuration loaded from the environment.
//!
//! [`Config::parse`] is pure — it reads through a `lookup` closure, so it is
//! unit-tested without touching the real environment. [`Config::from_env`] is
//! the thin production wrapper over `std::env::var`.

use std::time::Duration;

use anyhow::Context;

use crate::executor::ssh::SshConfig;

const DEFAULT_SSH_HOST: &str = "host.docker.internal";
const DEFAULT_SSH_PORT: u16 = 22;
const DEFAULT_SSH_USER: &str = "botuser";
const DEFAULT_SSH_KEY: &str = "/secrets/id_ed25519";
const DEFAULT_SSH_KNOWN_HOSTS: &str = "/secrets/known_hosts";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub allowed_chats: Vec<i64>,
    pub ssh: SshConfig,
    pub command_timeout: Duration,
}

impl Config {
    /// Load configuration from the process environment.
    pub fn from_env() -> anyhow::Result<Self> {
        Self::parse(|key| std::env::var(key).ok())
    }

    /// Parse configuration from a key → value lookup. A value present but empty
    /// is treated the same as missing.
    pub fn parse(lookup: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let get = |key: &str| lookup(key).filter(|v| !v.is_empty());

        let bot_token = get("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN is required")?;

        let allowed_chats = match get("TELEGRAM_ALLOWED_CHATS") {
            Some(list) => list
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.parse::<i64>()
                        .with_context(|| format!("invalid chat id {s:?} in TELEGRAM_ALLOWED_CHATS"))
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            None => Vec::new(),
        };

        let ssh = SshConfig {
            host: get("SSH_HOST").unwrap_or_else(|| DEFAULT_SSH_HOST.to_string()),
            port: parse_or("SSH_PORT", get("SSH_PORT"), DEFAULT_SSH_PORT)?,
            user: get("SSH_USER").unwrap_or_else(|| DEFAULT_SSH_USER.to_string()),
            key: get("SSH_KEY").unwrap_or_else(|| DEFAULT_SSH_KEY.to_string()),
            known_hosts: get("SSH_KNOWN_HOSTS")
                .unwrap_or_else(|| DEFAULT_SSH_KNOWN_HOSTS.to_string()),
        };

        let command_timeout = Duration::from_secs(parse_or(
            "COMMAND_TIMEOUT_SECS",
            get("COMMAND_TIMEOUT_SECS"),
            DEFAULT_TIMEOUT_SECS,
        )?);

        Ok(Config { bot_token, allowed_chats, ssh, command_timeout })
    }
}

/// Parse an optional raw value, falling back to `default` when it is absent.
fn parse_or<T: std::str::FromStr>(key: &str, raw: Option<String>, default: T) -> anyhow::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match raw {
        Some(raw) => raw.parse().with_context(|| format!("invalid {key} {raw:?}")),
        None => Ok(default),
    }
}
