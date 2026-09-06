use thiserror::Error;

/// Errors surfaced by the core crate.
///
/// These map cleanly onto user-facing messages in the UI (see `README` §Troubleshooting).
/// Technical detail is preserved for logs; the UI shows a friendly summary + "How to fix".
#[derive(Error, Debug)]
pub enum Error {
    /// `~/.claude/.credentials.json` (or `$CLAUDE_CONFIG_DIR`) does not exist.
    #[error("Claude Code not found: no credentials file at {0}")]
    CredentialsNotFound(String),

    /// The credentials file exists but has no usable `claudeAiOauth.accessToken`.
    #[error("Claude Code is not authenticated")]
    NotAuthenticated,

    /// The stored OAuth access token is past its `expiresAt`.
    #[error("Claude Code session token has expired")]
    TokenExpired,

    /// The usage endpoint returned HTTP 429 (rate limited). Callers should back off.
    #[error("Usage endpoint is rate limited (HTTP 429); try again later")]
    RateLimited,

    /// The usage endpoint returned an unauthorized response (token rejected).
    #[error("Usage endpoint rejected the token (HTTP {0})")]
    Unauthorized(u16),

    /// Any other non-success HTTP status.
    #[error("Usage endpoint returned HTTP {0}")]
    Http(u16),

    /// Network/transport failure.
    #[error("Network error: {0}")]
    Network(String),

    /// Failed to parse JSON (credentials, usage response or a transcript line).
    #[error("Parse error: {0}")]
    Parse(String),

    /// Filesystem error.
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
