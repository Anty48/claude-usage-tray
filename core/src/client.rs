//! The tiny HTTP client that queries the on-demand usage endpoint.
//!
//! Design constraints (from the product spec):
//! - **No background polling.** We only fetch when the user asks (click / Refresh).
//! - **Be gentle.** The endpoint is aggressively rate limited; we enforce a minimum
//!   interval between live fetches and surface 429 as [`Error::RateLimited`] so the UI can
//!   fall back to the last cached snapshot.
//! - **Correct headers.** The `User-Agent: claude-code/<ver>` and
//!   `anthropic-beta: oauth-2025-04-20` headers are required for the request to be accepted.

use std::time::Duration;

use crate::credentials::{self, Credentials};
use crate::error::{Error, Result};
use crate::usage::{self, UsageSnapshot};

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

/// User-Agent prefix required by the endpoint. The exact version is not important, but the
/// `claude-code/` prefix is: without it the request lands in a hostile rate-limit bucket.
pub fn user_agent() -> String {
    format!("claude-code/{}", option_env!("CLAUDE_CODE_UA_VERSION").unwrap_or("2.1.201"))
}

/// Current wall-clock time in epoch milliseconds (UTC).
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Fetch and normalize the current usage using locally stored Claude Code credentials.
///
/// This performs real network I/O and is therefore *not* exercised by unit tests; all of its
/// parsing/normalization logic lives in [`crate::usage`], which is fully tested offline.
pub fn fetch_usage() -> Result<UsageSnapshot> {
    let creds = credentials::load()?;
    fetch_usage_with(&creds)
}

/// Same as [`fetch_usage`] but with explicit credentials (useful for the app layer, which may
/// have already loaded them to show account info).
pub fn fetch_usage_with(creds: &Credentials) -> Result<UsageSnapshot> {
    if creds.is_expired(now_ms()) {
        return Err(Error::TokenExpired);
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .build();

    let response = agent
        .get(USAGE_URL)
        .set("Authorization", &format!("Bearer {}", creds.access_token))
        .set("anthropic-beta", ANTHROPIC_BETA)
        .set("User-Agent", &user_agent())
        .set("Accept", "application/json")
        .call();

    match response {
        Ok(resp) => {
            let body = resp
                .into_string()
                .map_err(|e| Error::Network(e.to_string()))?;
            usage::parse_usage(&body, now_ms(), creds.subscription_type.clone())
        }
        Err(ureq::Error::Status(code, _)) => Err(match code {
            429 => Error::RateLimited,
            401 | 403 => Error::Unauthorized(code),
            other => Error::Http(other),
        }),
        Err(ureq::Error::Transport(t)) => Err(Error::Network(t.to_string())),
    }
}

/// Minimum seconds between live fetches. The UI honors this to avoid hammering the endpoint;
/// within this window it shows the cached snapshot instead.
pub const MIN_FETCH_INTERVAL_SECS: i64 = 60;

/// Whether enough time has elapsed since `last_fetch_ms` to allow another live fetch.
pub fn may_fetch(last_fetch_ms: Option<i64>, now_ms: i64) -> bool {
    match last_fetch_ms {
        None => true,
        Some(prev) => (now_ms - prev) >= MIN_FETCH_INTERVAL_SECS * 1000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_has_required_prefix() {
        assert!(user_agent().starts_with("claude-code/"));
    }

    #[test]
    fn rate_guard_allows_first_fetch() {
        assert!(may_fetch(None, 1000));
    }

    #[test]
    fn rate_guard_blocks_within_interval() {
        let now = 1_000_000;
        assert!(!may_fetch(Some(now - 30_000), now)); // 30s < 60s
    }

    #[test]
    fn rate_guard_allows_after_interval() {
        let now = 1_000_000;
        assert!(may_fetch(Some(now - 61_000), now)); // 61s >= 60s
    }
}
