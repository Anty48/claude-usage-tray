//! Model catalog and relative-cost weighting used by the estimator.
//!
//! Different Claude models consume your rate-limit budget at very different rates. We cannot
//! know Anthropic's exact internal accounting, so these are **coarse relative weights** used
//! only to scale a heuristic estimate — never presented as exact figures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    Opus,
    #[default]
    Sonnet,
    Haiku,
}

impl Model {
    /// Relative burn multiplier against the reference model (Opus = 1.0).
    ///
    /// These reflect the rough public understanding that Sonnet is substantially cheaper than
    /// Opus and Haiku cheaper still. They are deliberately conservative and only shift the
    /// estimate's midpoint; they do not claim precision.
    pub fn burn_multiplier(self) -> f64 {
        match self {
            Model::Opus => 1.0,
            Model::Sonnet => 0.55,
            Model::Haiku => 0.25,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Model::Opus => "Opus",
            Model::Sonnet => "Sonnet",
            Model::Haiku => "Haiku",
        }
    }

    /// Best-effort mapping from a Claude Code model id / settings string.
    pub fn from_id(s: &str) -> Option<Model> {
        let s = s.to_ascii_lowercase();
        if s.contains("opus") {
            Some(Model::Opus)
        } else if s.contains("sonnet") {
            Some(Model::Sonnet)
        } else if s.contains("haiku") {
            Some(Model::Haiku)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipliers_are_ordered() {
        assert!(Model::Opus.burn_multiplier() > Model::Sonnet.burn_multiplier());
        assert!(Model::Sonnet.burn_multiplier() > Model::Haiku.burn_multiplier());
    }

    #[test]
    fn detects_model_from_settings_string() {
        assert_eq!(Model::from_id("claude-opus-4-8"), Some(Model::Opus));
        assert_eq!(Model::from_id("opus"), Some(Model::Opus));
        assert_eq!(Model::from_id("claude-sonnet-5"), Some(Model::Sonnet));
        assert_eq!(
            Model::from_id("claude-haiku-4-5-20251001"),
            Some(Model::Haiku)
        );
        assert_eq!(Model::from_id("gpt-4"), None);
    }
}
