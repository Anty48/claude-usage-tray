//! Tauri command handlers — the bridge between the webview UI and `claude-usage-core`.
//!
//! All heavy lifting lives in the core crate; these functions only translate types and hold a
//! tiny in-memory cache so the popup can paint instantly from the last snapshot while a fresh
//! fetch runs.

use std::sync::Mutex;

use claude_usage_core::{
    client,
    credentials,
    error::Error,
    estimator::{self, Complexity, EstimateInput, Sensitivity},
    history,
    models::Model,
    settings::Settings,
    usage::UsageSnapshot,
};
use serde::{Deserialize, Serialize};

/// In-memory app state (no secrets persisted).
#[derive(Default)]
pub struct AppState {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    last_snapshot: Option<UsageSnapshot>,
    last_fetch_ms: Option<i64>,
    /// When true, the user asked to ignore cached history for this run.
    history_cleared: bool,
}

/// Machine-readable error kind so the UI can show the right "How to fix" help.
fn error_kind(e: &Error) -> &'static str {
    match e {
        Error::CredentialsNotFound(_) => "not_found",
        Error::NotAuthenticated => "not_authenticated",
        Error::TokenExpired => "token_expired",
        Error::RateLimited => "rate_limited",
        Error::Unauthorized(_) => "unauthorized",
        Error::Http(_) | Error::Network(_) => "network",
        Error::Parse(_) | Error::Io(_) => "unavailable",
    }
}

#[derive(Serialize)]
pub struct UsageResponse {
    pub snapshot: Option<UsageSnapshot>,
    pub error: Option<String>,
    pub error_kind: Option<String>,
    pub from_cache: bool,
}

#[tauri::command]
pub fn get_usage(force: bool, state: tauri::State<'_, AppState>) -> UsageResponse {
    let now = client::now_ms();
    let (cached, last_fetch) = {
        let g = state.inner.lock().unwrap();
        (g.last_snapshot.clone(), g.last_fetch_ms)
    };

    // Honour the on-demand rate guard: within the min interval, serve cache (unless forced and
    // the guard still allows). We never poll in the background.
    if !force {
        if let Some(snap) = &cached {
            if !client::may_fetch(last_fetch, now) {
                return UsageResponse {
                    snapshot: Some(snap.clone()),
                    error: None,
                    error_kind: None,
                    from_cache: true,
                };
            }
        }
    }

    match client::fetch_usage() {
        Ok(snap) => {
            let mut g = state.inner.lock().unwrap();
            g.last_snapshot = Some(snap.clone());
            g.last_fetch_ms = Some(now);
            UsageResponse {
                snapshot: Some(snap),
                error: None,
                error_kind: None,
                from_cache: false,
            }
        }
        Err(e) => {
            let kind = error_kind(&e).to_string();
            // Fall back to cache when we have one (e.g. rate limited), but tell the UI.
            UsageResponse {
                snapshot: cached,
                error: Some(e.to_string()),
                error_kind: Some(kind),
                from_cache: true,
            }
        }
    }
}

#[derive(Serialize)]
pub struct AccountInfo {
    pub subscription_type: Option<String>,
    pub authenticated: bool,
}

#[tauri::command]
pub fn get_account() -> AccountInfo {
    match credentials::load() {
        Ok(c) => {
            let authenticated = !c.is_expired(client::now_ms());
            AccountInfo {
                subscription_type: c.subscription_type,
                authenticated,
            }
        }
        Err(_) => AccountInfo {
            subscription_type: None,
            authenticated: false,
        },
    }
}

// ----- Settings -----

#[tauri::command]
pub fn get_settings() -> Settings {
    Settings::load()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings.save().map_err(|e| e.to_string())
}

/// Best-effort detection of the model configured in Claude Code itself.
#[tauri::command]
pub fn detect_model() -> Option<String> {
    Settings::detect_claude_model().map(|m| m.label().to_lowercase())
}

// ----- History -----

#[derive(Serialize)]
pub struct HistoryResponse {
    pub sessions: u64,
    pub avg_messages: f64,
    pub avg_tokens: f64,
    pub total_tokens: u64,
    pub has_signal: bool,
    pub models: Vec<(String, u64)>,
    pub cleared: bool,
}

#[tauri::command]
pub fn get_history(state: tauri::State<'_, AppState>) -> HistoryResponse {
    let cleared = state.inner.lock().unwrap().history_cleared;
    if cleared {
        return HistoryResponse {
            sessions: 0,
            avg_messages: 0.0,
            avg_tokens: 0.0,
            total_tokens: 0,
            has_signal: false,
            models: vec![],
            cleared: true,
        };
    }
    let stats = history::aggregate_default(300);
    HistoryResponse {
        sessions: stats.sessions,
        avg_messages: (stats.avg_messages_per_session() * 10.0).round() / 10.0,
        avg_tokens: stats.avg_tokens_per_session().round(),
        total_tokens: stats.totals.total_tokens(),
        has_signal: stats.signal().is_some(),
        models: stats.totals.model_messages.into_iter().collect(),
        cleared: false,
    }
}

/// "Clear local history" only forgets OUR derived stats for this run. It never deletes Claude
/// Code's own transcripts — those belong to Claude Code.
#[tauri::command]
pub fn clear_history(state: tauri::State<'_, AppState>) {
    state.inner.lock().unwrap().history_cleared = true;
}

// ----- Estimator -----

#[derive(Deserialize)]
pub struct EstimateRequest {
    pub task: String,
    /// "small" | "medium" | "large" | "verylarge" | null (auto)
    pub complexity: Option<String>,
    /// "opus" | "sonnet" | "haiku"
    pub model: Option<String>,
    /// "conservative" | "balanced" | "aggressive"
    pub sensitivity: Option<String>,
    pub remaining_percent: f64,
    pub use_history: bool,
}

fn parse_complexity(s: &str) -> Option<Complexity> {
    match s.to_ascii_lowercase().as_str() {
        "small" => Some(Complexity::Small),
        "medium" => Some(Complexity::Medium),
        "large" => Some(Complexity::Large),
        "verylarge" | "very_large" | "very large" => Some(Complexity::VeryLarge),
        _ => None,
    }
}

fn parse_sensitivity(s: &str) -> Sensitivity {
    match s.to_ascii_lowercase().as_str() {
        "conservative" => Sensitivity::Conservative,
        "aggressive" => Sensitivity::Aggressive,
        _ => Sensitivity::Balanced,
    }
}

#[tauri::command]
pub fn estimate(
    req: EstimateRequest,
    state: tauri::State<'_, AppState>,
) -> estimator::Estimate {
    let model = req
        .model
        .as_deref()
        .and_then(Model::from_id)
        .unwrap_or_default();
    let sensitivity = req
        .sensitivity
        .as_deref()
        .map(parse_sensitivity)
        .unwrap_or_default();
    let complexity_override = req.complexity.as_deref().and_then(parse_complexity);

    let history = if req.use_history && !state.inner.lock().unwrap().history_cleared {
        history::aggregate_default(300).signal()
    } else {
        None
    };

    let input = EstimateInput {
        task: req.task,
        complexity_override,
        model,
        sensitivity,
        remaining_percent: req.remaining_percent,
        history,
    };
    estimator::estimate(&input)
}

// ----- OS helpers -----

/// Open a URL in the default browser (used by "How to fix" and About links).
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    // Only allow https to avoid opening arbitrary local handlers.
    if !url.starts_with("https://") {
        return Err("invalid url".into());
    }
    open_external(&url).map_err(|e| e.to_string())
}

/// "Open Claude Code": launch a fresh terminal running `claude`; if that fails, open the docs.
#[tauri::command]
pub fn open_claude_code() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        // `start` opens a new console window running claude.
        let launched = Command::new("cmd")
            .args(["/C", "start", "", "cmd", "/K", "claude"])
            .spawn();
        if launched.is_ok() {
            return Ok(());
        }
    }
    open_external("https://docs.claude.com/en/docs/claude-code/overview").map_err(|e| e.to_string())
}

/// Reflect current usage on the tray tooltip (Windows tray icons can't show a text badge, so
/// the remaining % is surfaced on hover). Called by the UI after a successful fetch.
#[tauri::command]
pub fn update_tray(app: tauri::AppHandle, remaining: Option<f64>, show_percent: bool) {
    if let Some(tray) = app.tray_by_id("main") {
        let tip = match (show_percent, remaining) {
            (true, Some(r)) => format!("Claude Usage — {:.0}% left this session", r),
            _ => "Claude Usage — click to open".to_string(),
        };
        let _ = tray.set_tooltip(Some(&tip));
    }
}

// ----- Autostart (Start with Windows) -----

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let m = app.autolaunch();
    let r = if enabled { m.enable() } else { m.disable() };
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

fn open_external(url: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
        Ok(())
    }
}
