//! Locate and read the Claude Code OAuth credentials that live on this machine.
//!
//! Claude Code stores an OAuth token locally after you sign in. On Windows this is a
//! plaintext JSON file at `%USERPROFILE%\.claude\.credentials.json`; the directory can be
//! overridden with the `CLAUDE_CONFIG_DIR` environment variable (matching Claude Code).
//!
//! **We never store, copy, ask for or transmit this token anywhere except the official
//! Anthropic usage endpoint** — exactly as Claude Code itself does. We also never write to
//! the file, so we cannot disturb Claude Code's own session.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

/// The subset of `.credentials.json` we care about.
#[derive(Debug, Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OauthBlock>,
}

#[derive(Debug, Deserialize)]
struct OauthBlock {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    /// Milliseconds since the Unix epoch.
    #[serde(rename = "expiresAt")]
    expires_at: Option<i64>,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
    #[serde(rename = "rateLimitTier")]
    rate_limit_tier: Option<String>,
}

/// A live token ready to authenticate the usage request.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub access_token: String,
    /// Milliseconds since epoch, if known.
    pub expires_at_ms: Option<i64>,
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
}

impl Credentials {
    /// Returns true when we have an expiry and it is already in the past.
    pub fn is_expired(&self, now_ms: i64) -> bool {
        matches!(self.expires_at_ms, Some(exp) if exp <= now_ms)
    }
}

/// Directory that holds Claude Code's config (`~/.claude` by default).
pub fn claude_config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    dirs::home_dir().map(|h| h.join(".claude"))
}

/// Full path to the credentials file we will read.
pub fn credentials_path() -> Option<PathBuf> {
    claude_config_dir().map(|d| d.join(".credentials.json"))
}

/// Read and parse the credentials from the default location.
pub fn load() -> Result<Credentials> {
    let path =
        credentials_path().ok_or_else(|| Error::CredentialsNotFound("<no home dir>".into()))?;
    load_from(&path)
}

/// Read and parse the credentials from an explicit path (used by tests).
pub fn load_from(path: &Path) -> Result<Credentials> {
    if !path.exists() {
        return Err(Error::CredentialsNotFound(path.display().to_string()));
    }
    let raw = std::fs::read_to_string(path)?;
    parse(&raw)
}

/// Parse the credentials JSON text. Separated out so it is trivially testable.
pub fn parse(raw: &str) -> Result<Credentials> {
    let file: CredentialsFile = serde_json::from_str(raw)?;
    let block = file.claude_ai_oauth.ok_or(Error::NotAuthenticated)?;
    let access_token = block
        .access_token
        .filter(|t| !t.trim().is_empty())
        .ok_or(Error::NotAuthenticated)?;
    Ok(Credentials {
        access_token,
        expires_at_ms: block.expires_at,
        subscription_type: block.subscription_type,
        rate_limit_tier: block.rate_limit_tier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "claudeAiOauth": {
            "accessToken": "sk-ant-oat-TESTTOKEN",
            "refreshToken": "sk-ant-ort-SECRET",
            "expiresAt": 1757212542723,
            "scopes": ["user:inference"],
            "subscriptionType": "max",
            "rateLimitTier": "default_max_20x"
        }
    }"#;

    #[test]
    fn parses_valid_credentials() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.access_token, "sk-ant-oat-TESTTOKEN");
        assert_eq!(c.expires_at_ms, Some(1757212542723));
        assert_eq!(c.subscription_type.as_deref(), Some("max"));
        assert_eq!(c.rate_limit_tier.as_deref(), Some("default_max_20x"));
    }

    #[test]
    fn detects_expiry() {
        let c = parse(SAMPLE).unwrap();
        assert!(c.is_expired(1757212542724));
        assert!(!c.is_expired(1757212542722));
    }

    #[test]
    fn missing_oauth_block_is_not_authenticated() {
        let err = parse(r#"{"other": true}"#).unwrap_err();
        assert!(matches!(err, Error::NotAuthenticated));
    }

    #[test]
    fn empty_token_is_not_authenticated() {
        let err = parse(r#"{"claudeAiOauth":{"accessToken":""}}"#).unwrap_err();
        assert!(matches!(err, Error::NotAuthenticated));
    }

    #[test]
    fn garbage_is_parse_error() {
        let err = parse("not json").unwrap_err();
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn missing_file_reports_not_found() {
        let err = load_from(Path::new("/definitely/does/not/exist.json")).unwrap_err();
        assert!(matches!(err, Error::CredentialsNotFound(_)));
    }
}
