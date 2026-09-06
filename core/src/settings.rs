//! Persisted user preferences.
//!
//! Stored as a small JSON file in the OS config dir
//! (`%APPDATA%\ClaudeUsageTray\settings.json` on Windows). Contains **no secrets** — only UI
//! preferences. The Claude token is never written here or anywhere by this app.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::estimator::Sensitivity;
use crate::models::Model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Register the app to launch on login. Default OFF (per spec).
    pub start_with_windows: bool,
    /// Show the remaining % as a badge on the tray icon.
    pub show_percentage_in_tray: bool,
    /// Fetch fresh data when the popup is opened. Default ON (per spec).
    pub refresh_on_click: bool,
    pub theme: Theme,
    /// Model assumed by the calculator (auto-detected from Claude Code settings when possible).
    pub model: Model,
    pub sensitivity: Sensitivity,
    /// Use the user's local transcript history to calibrate estimates. Default ON.
    pub use_history: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            start_with_windows: false,
            show_percentage_in_tray: true,
            refresh_on_click: true,
            theme: Theme::System,
            model: Model::default(),
            sensitivity: Sensitivity::default(),
            use_history: true,
        }
    }
}

impl Settings {
    /// Path to the settings file (`<config>/ClaudeUsageTray/settings.json`).
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ClaudeUsageTray").join("settings.json"))
    }

    /// Load settings, falling back to defaults on any error (missing file, bad JSON).
    pub fn load() -> Settings {
        match Self::path().and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(raw) => Self::from_json(&raw).unwrap_or_default(),
            None => Settings::default(),
        }
    }

    /// Parse from JSON. Unknown/missing fields fall back to defaults thanks to `#[serde(default)]`.
    pub fn from_json(raw: &str) -> crate::Result<Settings> {
        Ok(serde_json::from_str(raw)?)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    /// Persist to disk, creating the directory if needed.
    pub fn save(&self) -> crate::Result<()> {
        if let Some(path) = Self::path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, self.to_json())?;
        }
        Ok(())
    }

    /// Best-effort: read the model configured in Claude Code's own `settings.json`.
    pub fn detect_claude_model() -> Option<Model> {
        let path = crate::credentials::claude_config_dir()?.join("settings.json");
        let raw = std::fs::read_to_string(path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        v.get("model").and_then(|m| m.as_str()).and_then(Model::from_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_spec() {
        let s = Settings::default();
        assert!(!s.start_with_windows);
        assert!(s.refresh_on_click);
        assert_eq!(s.theme, Theme::System);
    }

    #[test]
    fn round_trips_through_json() {
        let mut s = Settings::default();
        s.theme = Theme::Dark;
        s.model = Model::Opus;
        s.start_with_windows = true;
        let json = s.to_json();
        let back = Settings::from_json(&json).unwrap();
        assert_eq!(back.theme, Theme::Dark);
        assert_eq!(back.model, Model::Opus);
        assert!(back.start_with_windows);
    }

    #[test]
    fn partial_json_uses_defaults() {
        let back = Settings::from_json(r#"{"theme":"light"}"#).unwrap();
        assert_eq!(back.theme, Theme::Light);
        assert!(back.refresh_on_click); // default preserved
    }

    #[test]
    fn empty_object_is_all_defaults() {
        let back = Settings::from_json("{}").unwrap();
        assert_eq!(back.theme, Theme::System);
    }
}
