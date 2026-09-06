//! Types for the Anthropic OAuth usage endpoint and normalization into a small,
//! UI-friendly [`UsageSnapshot`].
//!
//! The raw endpoint (`/api/oauth/usage`) returns a fairly large object. We only depend on
//! the documented, stable fields (`five_hour`, `seven_day`, `seven_day_opus`,
//! `seven_day_sonnet`) plus the normalized `limits[]` array when present. Unknown/codename
//! buckets are ignored on purpose so a server change cannot break parsing.
//!
//! Note on semantics: `utilization` / `percent` is the **fraction used** (0–100).
//! "Remaining" is therefore `100 - used`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Traffic-light state derived from how much of a window is *left*.
///
/// Thresholds follow the product spec (on remaining %):
/// `>50 Normal`, `25–50 WarnSoft`, `10–25 Warn`, `<10 Critical`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Normal,
    WarnSoft,
    Warn,
    Critical,
}

impl Severity {
    /// Classify from the percentage *remaining* (0–100).
    pub fn from_remaining(remaining: f64) -> Self {
        if remaining < 10.0 {
            Severity::Critical
        } else if remaining < 25.0 {
            Severity::Warn
        } else if remaining < 50.0 {
            Severity::WarnSoft
        } else {
            Severity::Normal
        }
    }
}

/// One usage window (session / weekly / per-model), normalized for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct UsageWindow {
    /// Stable machine key, e.g. `session`, `weekly_all`, `weekly_opus`, `weekly_sonnet`.
    pub key: String,
    /// Human label, e.g. "Current session (5h)".
    pub label: String,
    /// Percentage used, 0–100.
    pub percent_used: f64,
    /// Percentage remaining, 0–100.
    pub percent_remaining: f64,
    /// When this window resets, if known.
    pub resets_at: Option<DateTime<Utc>>,
    /// Traffic-light state from remaining %.
    pub severity: Severity,
    /// Whether the server considers this window currently active.
    pub is_active: bool,
}

impl UsageWindow {
    fn new(key: &str, label: &str, percent_used: f64, resets_at: Option<DateTime<Utc>>) -> Self {
        let used = percent_used.clamp(0.0, 100.0);
        let remaining = 100.0 - used;
        UsageWindow {
            key: key.to_string(),
            label: label.to_string(),
            percent_used: used,
            percent_remaining: remaining,
            resets_at,
            severity: Severity::from_remaining(remaining),
            is_active: true,
        }
    }
}

/// Paid overflow credits ("extra usage").
#[derive(Debug, Clone, Serialize, Default)]
pub struct ExtraUsage {
    pub enabled: bool,
    pub used_credits: Option<f64>,
    pub monthly_limit: Option<f64>,
    pub currency: Option<String>,
    pub spend_percent: Option<f64>,
}

/// The normalized snapshot the UI consumes.
#[derive(Debug, Clone, Serialize)]
pub struct UsageSnapshot {
    /// The 5-hour session window (the headline figure), if present.
    pub session: Option<UsageWindow>,
    /// The combined weekly window, if present.
    pub weekly: Option<UsageWindow>,
    /// Any additional windows (per-model weekly, etc.).
    pub extras: Vec<UsageWindow>,
    /// Paid overflow credits, if the account has them.
    pub extra_usage: Option<ExtraUsage>,
    /// When we fetched this (epoch millis, UTC).
    pub fetched_at_ms: i64,
    /// The account subscription type, if known from credentials.
    pub subscription_type: Option<String>,
}

impl UsageSnapshot {
    /// Overall severity = worst of session + weekly (what the tray icon reflects).
    pub fn overall_severity(&self) -> Severity {
        let mut worst = Severity::Normal;
        for w in self.session.iter().chain(self.weekly.iter()) {
            worst = worst_of(worst, w.severity);
        }
        worst
    }

    /// The headline "remaining" percent the tray tooltip shows (session first).
    pub fn headline_remaining(&self) -> Option<f64> {
        self.session
            .as_ref()
            .or(self.weekly.as_ref())
            .map(|w| w.percent_remaining)
    }
}

fn worst_of(a: Severity, b: Severity) -> Severity {
    fn rank(s: Severity) -> u8 {
        match s {
            Severity::Normal => 0,
            Severity::WarnSoft => 1,
            Severity::Warn => 2,
            Severity::Critical => 3,
        }
    }
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

// ----- Raw endpoint shapes (only the fields we rely on) ----------------------

#[derive(Debug, Deserialize)]
struct RawBucket {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawExtraUsage {
    is_enabled: Option<bool>,
    monthly_limit: Option<f64>,
    used_credits: Option<f64>,
    currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSpend {
    percent: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawUsage {
    five_hour: Option<RawBucket>,
    seven_day: Option<RawBucket>,
    seven_day_opus: Option<RawBucket>,
    seven_day_sonnet: Option<RawBucket>,
    extra_usage: Option<RawExtraUsage>,
    spend: Option<RawSpend>,
}

fn parse_ts(s: &Option<String>) -> Option<DateTime<Utc>> {
    s.as_ref()
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

/// Parse the raw usage JSON into a normalized [`UsageSnapshot`].
///
/// `fetched_at_ms` is injected by the caller so this stays pure/deterministic in tests.
pub fn parse_usage(
    raw: &str,
    fetched_at_ms: i64,
    subscription_type: Option<String>,
) -> crate::Result<UsageSnapshot> {
    let r: RawUsage = serde_json::from_str(raw)?;

    let session = r.five_hour.as_ref().and_then(|b| {
        b.utilization
            .map(|u| UsageWindow::new("session", "Current session (5h)", u, parse_ts(&b.resets_at)))
    });
    let weekly = r.seven_day.as_ref().and_then(|b| {
        b.utilization
            .map(|u| UsageWindow::new("weekly_all", "This week (all models)", u, parse_ts(&b.resets_at)))
    });

    let mut extras = Vec::new();
    if let Some(b) = r.seven_day_opus.as_ref() {
        if let Some(u) = b.utilization {
            extras.push(UsageWindow::new(
                "weekly_opus",
                "This week (Opus)",
                u,
                parse_ts(&b.resets_at),
            ));
        }
    }
    if let Some(b) = r.seven_day_sonnet.as_ref() {
        if let Some(u) = b.utilization {
            extras.push(UsageWindow::new(
                "weekly_sonnet",
                "This week (Sonnet)",
                u,
                parse_ts(&b.resets_at),
            ));
        }
    }

    let extra_usage = r.extra_usage.as_ref().map(|e| ExtraUsage {
        enabled: e.is_enabled.unwrap_or(false),
        used_credits: e.used_credits,
        monthly_limit: e.monthly_limit,
        currency: e.currency.clone(),
        spend_percent: r.spend.as_ref().and_then(|s| s.percent),
    });

    Ok(UsageSnapshot {
        session,
        weekly,
        extras,
        extra_usage,
        fetched_at_ms,
        subscription_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed real payload shape from GET /api/oauth/usage.
    const SAMPLE: &str = r#"{
        "five_hour": {"utilization": 67, "resets_at": "2026-09-06T23:50:00.414269+00:00"},
        "seven_day": {"utilization": 13, "resets_at": "2026-09-12T00:00:00.414290+00:00"},
        "seven_day_opus": null,
        "seven_day_sonnet": {"utilization": 4, "resets_at": "2026-09-11T03:00:00+00:00"},
        "extra_usage": {"is_enabled": true, "monthly_limit": 8000, "used_credits": 0, "currency": "EUR"},
        "spend": {"percent": 0}
    }"#;

    #[test]
    fn severity_thresholds_match_spec() {
        assert_eq!(Severity::from_remaining(80.0), Severity::Normal);
        assert_eq!(Severity::from_remaining(50.0), Severity::Normal);
        assert_eq!(Severity::from_remaining(49.9), Severity::WarnSoft);
        assert_eq!(Severity::from_remaining(25.0), Severity::WarnSoft);
        assert_eq!(Severity::from_remaining(24.9), Severity::Warn);
        assert_eq!(Severity::from_remaining(10.0), Severity::Warn);
        assert_eq!(Severity::from_remaining(9.9), Severity::Critical);
    }

    #[test]
    fn parses_session_and_weekly() {
        let s = parse_usage(SAMPLE, 1_700_000_000_000, Some("max".into())).unwrap();
        let session = s.session.unwrap();
        assert_eq!(session.percent_used, 67.0);
        assert_eq!(session.percent_remaining, 33.0);
        assert_eq!(session.severity, Severity::WarnSoft);
        assert!(session.resets_at.is_some());

        let weekly = s.weekly.unwrap();
        assert_eq!(weekly.percent_used, 13.0);
        assert_eq!(weekly.severity, Severity::Normal);
    }

    #[test]
    fn per_model_extras_are_included_when_present() {
        let s = parse_usage(SAMPLE, 0, None).unwrap();
        // Opus is null -> skipped; Sonnet present -> included.
        assert_eq!(s.extras.len(), 1);
        assert_eq!(s.extras[0].key, "weekly_sonnet");
        assert_eq!(s.extras[0].percent_used, 4.0);
    }

    #[test]
    fn extra_usage_credits_parsed() {
        let s = parse_usage(SAMPLE, 0, None).unwrap();
        let eu = s.extra_usage.unwrap();
        assert!(eu.enabled);
        assert_eq!(eu.monthly_limit, Some(8000.0));
        assert_eq!(eu.currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn overall_severity_is_worst_of_windows() {
        let s = parse_usage(SAMPLE, 0, None).unwrap();
        // session WarnSoft, weekly Normal -> WarnSoft
        assert_eq!(s.overall_severity(), Severity::WarnSoft);
    }

    #[test]
    fn headline_remaining_prefers_session() {
        let s = parse_usage(SAMPLE, 0, None).unwrap();
        assert_eq!(s.headline_remaining(), Some(33.0));
    }

    #[test]
    fn tolerates_unknown_and_missing_fields() {
        let json = r#"{"five_hour":{"utilization":5,"resets_at":null},"mystery_bucket":{"x":1}}"#;
        let s = parse_usage(json, 0, None).unwrap();
        assert_eq!(s.session.unwrap().percent_used, 5.0);
        assert!(s.weekly.is_none());
    }
}
