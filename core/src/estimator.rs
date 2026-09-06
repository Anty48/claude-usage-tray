//! Heuristic "Can my prompt finish?" estimator.
//!
//! **This is deliberately imprecise and says so.** Real Claude Code consumption depends on
//! message count, context size, files read, tools used, iterations, retries, response length
//! and the model — none of which are knowable in advance. So we never output a single number;
//! we output a **range**, a **confidence level** (capped at Medium), the **factors** that drove
//! the estimate, and a **probability of finishing** derived from your remaining budget.
//!
//! Everything here is pure and deterministic, so it is exhaustively unit-tested.

use serde::{Deserialize, Serialize};

use crate::models::Model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Small,
    Medium,
    Large,
    VeryLarge,
}

impl Complexity {
    /// Base cost range in **session-% for the reference model (Opus)**.
    fn base_range(self) -> (f64, f64) {
        match self {
            Complexity::Small => (2.0, 7.0),
            Complexity::Medium => (7.0, 18.0),
            Complexity::Large => (18.0, 42.0),
            Complexity::VeryLarge => (38.0, 75.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Complexity::Small => "Small",
            Complexity::Medium => "Medium",
            Complexity::Large => "Large",
            Complexity::VeryLarge => "Very Large",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    /// Assume tasks cost more (earlier warnings).
    Conservative,
    Balanced,
    /// Assume tasks cost less.
    Aggressive,
}

impl Sensitivity {
    fn factor(self) -> f64 {
        match self {
            Sensitivity::Conservative => 1.2,
            Sensitivity::Balanced => 1.0,
            Sensitivity::Aggressive => 0.85,
        }
    }
}

impl Default for Sensitivity {
    fn default() -> Self {
        Sensitivity::Balanced
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    LikelyToFinish,
    Borderline,
    RiskOfRunningOut,
}

/// Optional signal derived from the user's real history (see [`crate::history`]).
/// `relative_size` is 1.0 for a "typical" session; >1 means this user's sessions tend to be
/// larger than the baseline, so estimates are nudged up.
#[derive(Debug, Clone, Copy)]
pub struct HistorySignal {
    pub relative_size: f64,
}

/// Input to the estimator.
#[derive(Debug, Clone)]
pub struct EstimateInput {
    pub task: String,
    /// `None` = auto-detect from the task text.
    pub complexity_override: Option<Complexity>,
    pub model: Model,
    pub sensitivity: Sensitivity,
    /// Session budget remaining (0–100).
    pub remaining_percent: f64,
    pub history: Option<HistorySignal>,
}

/// Output of the estimator.
#[derive(Debug, Clone, Serialize)]
pub struct Estimate {
    pub complexity: Complexity,
    /// Whether complexity was auto-detected (vs user-selected).
    pub auto_detected: bool,
    /// Estimated usage required, low end (session %).
    pub usage_min: f64,
    /// Estimated usage required, high end (session %).
    pub usage_max: f64,
    pub remaining_percent: f64,
    /// Probability of finishing (0–100), from remaining vs the estimate range.
    pub finish_probability: f64,
    /// remaining - midpoint of estimate (can be negative).
    pub margin: f64,
    pub verdict: Verdict,
    pub confidence: Confidence,
    /// Human-readable drivers of the estimate, shown in the UI.
    pub factors: Vec<String>,
}

// ----- Keyword lexicon -------------------------------------------------------

/// (keyword, weight). Positive = larger; negative = smaller. Iteration words also add spread.
const LEXICON: &[(&str, i32)] = &[
    // large scope
    ("entire", 3),
    ("whole", 3),
    ("complete", 2),
    ("everything", 3),
    ("codebase", 3),
    ("across", 2),
    ("migrate", 3),
    ("migration", 3),
    ("rewrite", 3),
    ("refactor", 3),
    ("architecture", 3),
    ("redesign", 3),
    ("overhaul", 3),
    ("all files", 3),
    ("multiple files", 2),
    ("every", 2),
    // medium build work
    ("implement", 2),
    ("feature", 2),
    ("integrate", 2),
    ("endpoint", 2),
    ("component", 1),
    ("module", 1),
    ("build", 1),
    ("create", 1),
    ("add", 1),
    // iterative / debugging (uncertainty)
    ("debug", 2),
    ("investigate", 2),
    ("reproduce", 2),
    ("failing", 2),
    ("flaky", 2),
    ("tests", 1),
    ("test suite", 2),
    // small
    ("typo", -3),
    ("rename", -2),
    ("one line", -3),
    ("one-line", -3),
    ("quick", -2),
    ("minor", -2),
    ("small", -2),
    ("tweak", -2),
    ("format", -1),
    ("lint", -1),
    ("comment", -1),
];

/// Detect complexity from free text, returning (complexity, matched factors, uncertainty spread).
fn detect_complexity(task: &str) -> (Complexity, Vec<String>, f64, usize) {
    let text = task.to_ascii_lowercase();
    let word_count = task.split_whitespace().count();

    let mut score: i32 = 0;
    let mut factors: Vec<String> = Vec::new();
    let mut spread: f64 = 0.0;
    let mut matches = 0usize;

    // Length contributes to scope.
    let len_bonus = match word_count {
        0..=14 => 0,
        15..=40 => 1,
        41..=80 => 2,
        _ => 3,
    };
    score += len_bonus;
    if len_bonus >= 2 {
        factors.push("Long, detailed description → wider scope".into());
    }

    for (kw, w) in LEXICON {
        if text.contains(kw) {
            score += w;
            matches += 1;
            if *w >= 3 {
                factors.push(format!("Mentions “{kw}” → large scope"));
            } else if *w == 2 && matches!(*kw, "debug" | "investigate" | "reproduce" | "failing" | "flaky" | "test suite") {
                factors.push(format!("Mentions “{kw}” → iterative, harder to bound"));
                spread += 6.0;
            } else if *w > 0 {
                factors.push(format!("Mentions “{kw}”"));
            } else {
                factors.push(format!("Mentions “{kw}” → likely small"));
            }
        }
    }

    let complexity = if score <= 1 {
        Complexity::Small
    } else if score <= 3 {
        Complexity::Medium
    } else if score <= 6 {
        Complexity::Large
    } else {
        Complexity::VeryLarge
    };

    (complexity, factors, spread, matches)
}

/// Run the estimator.
pub fn estimate(input: &EstimateInput) -> Estimate {
    let (auto_complexity, mut factors, spread, matches) = detect_complexity(&input.task);
    let auto_detected = input.complexity_override.is_none();
    let complexity = input.complexity_override.unwrap_or(auto_complexity);

    if !auto_detected {
        factors.insert(0, format!("Complexity set manually to {}", complexity.label()));
    }

    let (mut lo, mut hi) = complexity.base_range();

    // Model burn weighting.
    let m = input.model.burn_multiplier();
    lo *= m;
    hi *= m;
    if input.model != Model::Opus {
        factors.push(format!(
            "Model: {} → lower burn than Opus (×{:.2})",
            input.model.label(),
            m
        ));
    } else {
        factors.push("Model: Opus → highest burn".into());
    }

    // Iteration/debugging spread widens the high end.
    hi += spread * m;

    // History nudge.
    if let Some(h) = input.history {
        if h.relative_size > 1.25 {
            lo *= 1.15;
            hi *= 1.2;
            factors.push("Your recent sessions run larger than average → nudged up".into());
        } else if h.relative_size < 0.8 {
            lo *= 0.9;
            hi *= 0.92;
            factors.push("Your recent sessions run smaller than average → nudged down".into());
        }
    }

    // Sensitivity.
    let s = input.sensitivity.factor();
    lo *= s;
    hi *= s;

    // Clamp to sane bounds.
    lo = lo.clamp(0.5, 100.0);
    hi = hi.clamp(lo + 1.0, 120.0);

    let remaining = input.remaining_percent.clamp(0.0, 100.0);

    // Probability of finishing: treat required usage as uniform over [lo, hi].
    let finish_probability = if remaining >= hi {
        100.0
    } else if remaining <= lo {
        0.0
    } else {
        ((remaining - lo) / (hi - lo) * 100.0).clamp(0.0, 100.0)
    };

    let midpoint = (lo + hi) / 2.0;
    let margin = remaining - midpoint;

    let verdict = if finish_probability >= 60.0 {
        Verdict::LikelyToFinish
    } else if finish_probability <= 35.0 {
        Verdict::RiskOfRunningOut
    } else {
        Verdict::Borderline
    };

    // Confidence: never "High" — the spec forbids selling false precision.
    let confidence = if !auto_detected || matches >= 2 || input.task.split_whitespace().count() > 25
    {
        Confidence::Medium
    } else {
        Confidence::Low
    };

    Estimate {
        complexity,
        auto_detected,
        usage_min: round1(lo),
        usage_max: round1(hi),
        remaining_percent: remaining,
        finish_probability: round1(finish_probability),
        margin: round1(margin),
        verdict,
        confidence,
        factors,
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(task: &str, remaining: f64) -> EstimateInput {
        EstimateInput {
            task: task.into(),
            complexity_override: None,
            model: Model::Opus,
            sensitivity: Sensitivity::Balanced,
            remaining_percent: remaining,
            history: None,
        }
    }

    #[test]
    fn detects_small_task() {
        let (c, _, _, _) = detect_complexity("fix a typo in the readme");
        assert_eq!(c, Complexity::Small);
    }

    #[test]
    fn detects_large_task() {
        let (c, _, _, _) =
            detect_complexity("refactor the entire codebase and migrate architecture across modules");
        assert_eq!(c, Complexity::VeryLarge);
    }

    #[test]
    fn small_task_with_plenty_of_budget_is_likely() {
        let e = estimate(&input("fix a small typo", 80.0));
        assert_eq!(e.complexity, Complexity::Small);
        assert_eq!(e.verdict, Verdict::LikelyToFinish);
        assert!(e.finish_probability > 90.0);
    }

    #[test]
    fn huge_task_with_little_budget_is_risky() {
        let e = estimate(&input(
            "rewrite the entire codebase, migrate the whole architecture and debug all failing tests across every module",
            15.0,
        ));
        assert_eq!(e.complexity, Complexity::VeryLarge);
        assert_eq!(e.verdict, Verdict::RiskOfRunningOut);
        assert!(e.finish_probability < 35.0);
    }

    #[test]
    fn manual_override_wins_and_is_flagged() {
        let mut i = input("do a thing", 50.0);
        i.complexity_override = Some(Complexity::VeryLarge);
        let e = estimate(&i);
        assert_eq!(e.complexity, Complexity::VeryLarge);
        assert!(!e.auto_detected);
        assert!(e.factors.iter().any(|f| f.contains("manually")));
    }

    #[test]
    fn cheaper_model_lowers_estimate() {
        let mut opus = input("implement a new feature endpoint and component", 100.0);
        let opus_e = estimate(&opus);
        opus.model = Model::Haiku;
        let haiku_e = estimate(&opus);
        assert!(haiku_e.usage_max < opus_e.usage_max);
    }

    #[test]
    fn conservative_sensitivity_raises_estimate() {
        let mut i = input("implement a feature", 100.0);
        let balanced = estimate(&i).usage_max;
        i.sensitivity = Sensitivity::Conservative;
        let conservative = estimate(&i).usage_max;
        assert!(conservative > balanced);
    }

    #[test]
    fn probability_is_monotonic_in_remaining() {
        let low = estimate(&input("implement a medium feature", 20.0)).finish_probability;
        let high = estimate(&input("implement a medium feature", 90.0)).finish_probability;
        assert!(high >= low);
    }

    #[test]
    fn confidence_never_exceeds_medium() {
        let e = estimate(&input(
            "refactor the entire codebase and migrate everything across all modules and files",
            50.0,
        ));
        assert_eq!(e.confidence, Confidence::Medium);
    }

    #[test]
    fn history_nudges_estimate_up() {
        let mut i = input("implement a feature", 100.0);
        let base = estimate(&i).usage_max;
        i.history = Some(HistorySignal { relative_size: 1.5 });
        let nudged = estimate(&i).usage_max;
        assert!(nudged > base);
    }

    #[test]
    fn estimate_ranges_stay_bounded() {
        let e = estimate(&input(
            "rewrite entire codebase migrate whole architecture overhaul everything across every module and file",
            0.0,
        ));
        assert!(e.usage_min >= 0.5);
        assert!(e.usage_max <= 120.0);
        assert!(e.usage_max > e.usage_min);
    }
}
