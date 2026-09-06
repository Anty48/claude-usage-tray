//! Local, read-only statistics derived from real Claude Code transcripts.
//!
//! Claude Code stores every conversation as JSON Lines under
//! `~/.claude/projects/<slug>/<session-id>.jsonl`. Assistant messages carry a `usage` object
//! (`input_tokens`, `output_tokens`, `cache_*`) and a `model`. We read these **locally and
//! read-only** to show the user real numbers and to lightly calibrate the estimator.
//!
//! We deliberately do **not** claim a token → rate-limit-% mapping (Anthropic does not expose
//! one); history is used only for relative sizing and honest reporting.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::credentials::claude_config_dir;
use crate::estimator::HistorySignal;

/// A rough "typical" total-token size for a single Claude Code session, used only to derive a
/// relative-size signal for the estimator. Not a claim about anyone's real usage.
const TYPICAL_SESSION_TOKENS: f64 = 250_000.0;

#[derive(Debug, Clone, Default, Serialize)]
pub struct SessionStats {
    pub assistant_messages: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    /// message count per model id.
    pub model_messages: BTreeMap<String, u64>,
}

impl SessionStats {
    /// input + output + cache creation + cache read.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }

    fn merge(&mut self, other: &SessionStats) {
        self.assistant_messages += other.assistant_messages;
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_tokens += other.cache_creation_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        for (k, v) in &other.model_messages {
            *self.model_messages.entry(k.clone()).or_insert(0) += v;
        }
    }
}

/// Aggregate statistics across many sessions.
#[derive(Debug, Clone, Default, Serialize)]
pub struct HistoryStats {
    pub sessions: u64,
    pub totals: SessionStats,
}

impl HistoryStats {
    pub fn avg_tokens_per_session(&self) -> f64 {
        if self.sessions == 0 {
            0.0
        } else {
            self.totals.total_tokens() as f64 / self.sessions as f64
        }
    }

    pub fn avg_messages_per_session(&self) -> f64 {
        if self.sessions == 0 {
            0.0
        } else {
            self.totals.assistant_messages as f64 / self.sessions as f64
        }
    }

    /// A relative-size signal for the estimator, or `None` when there isn't enough data.
    pub fn signal(&self) -> Option<HistorySignal> {
        if self.sessions < 3 {
            return None;
        }
        let rel = (self.avg_tokens_per_session() / TYPICAL_SESSION_TOKENS).clamp(0.5, 2.0);
        Some(HistorySignal { relative_size: rel })
    }
}

/// Parse one session transcript (an iterator of JSONL lines) into [`SessionStats`].
///
/// Malformed lines are skipped; lines without assistant usage are ignored.
pub fn parse_session_lines<'a, I: IntoIterator<Item = &'a str>>(lines: I) -> SessionStats {
    let mut stats = SessionStats::default();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let msg = match v.get("message") {
            Some(m) => m,
            None => continue,
        };
        let usage = match msg.get("usage") {
            Some(u) => u,
            None => continue,
        };
        stats.assistant_messages += 1;
        stats.input_tokens += usage.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
        stats.output_tokens += usage.get("output_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
        stats.cache_creation_tokens += usage
            .get("cache_creation_input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        stats.cache_read_tokens += usage
            .get("cache_read_input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        if let Some(model) = msg.get("model").and_then(|m| m.as_str()) {
            *stats.model_messages.entry(model.to_string()).or_insert(0) += 1;
        }
    }
    stats
}

/// Default projects directory (`~/.claude/projects`).
pub fn projects_dir() -> Option<PathBuf> {
    claude_config_dir().map(|d| d.join("projects"))
}

/// Scan every `*.jsonl` transcript under `dir` and aggregate statistics.
///
/// `max_sessions` bounds the work (newest first) so this stays fast even with a huge history.
pub fn aggregate_dir(dir: &Path, max_sessions: usize) -> HistoryStats {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    collect_jsonl(dir, &mut files);
    // Newest first.
    files.sort_by(|a, b| b.0.cmp(&a.0));

    let mut stats = HistoryStats::default();
    for (_, path) in files.into_iter().take(max_sessions) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let session = parse_session_lines(content.lines());
            if session.assistant_messages == 0 {
                continue;
            }
            stats.sessions += 1;
            stats.totals.merge(&session);
        }
    }
    stats
}

fn collect_jsonl(dir: &Path, out: &mut Vec<(std::time::SystemTime, PathBuf)>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            out.push((mtime, path));
        }
    }
}

/// Convenience: aggregate the default projects dir.
pub fn aggregate_default(max_sessions: usize) -> HistoryStats {
    match projects_dir() {
        Some(d) => aggregate_dir(&d, max_sessions),
        None => HistoryStats::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINES: &str = r#"{"type":"user","message":{"role":"user","content":"hi"}}
{"type":"assistant","message":{"model":"claude-opus-4-8","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":200,"cache_read_input_tokens":10}}}
{"type":"attachment","foo":true}
{"type":"assistant","message":{"model":"claude-sonnet-5","usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":1000}}}
not valid json at all
{"type":"assistant","message":{"model":"claude-opus-4-8"}}"#;

    #[test]
    fn parses_usage_and_skips_noise() {
        let s = parse_session_lines(LINES.lines());
        // 2 assistant messages have usage; the third assistant has no usage and is skipped.
        assert_eq!(s.assistant_messages, 2);
        assert_eq!(s.input_tokens, 110);
        assert_eq!(s.output_tokens, 55);
        assert_eq!(s.cache_creation_tokens, 200);
        assert_eq!(s.cache_read_tokens, 1010);
        assert_eq!(s.total_tokens(), 110 + 55 + 200 + 1010);
    }

    #[test]
    fn tracks_per_model_counts() {
        let s = parse_session_lines(LINES.lines());
        assert_eq!(s.model_messages.get("claude-opus-4-8"), Some(&1));
        assert_eq!(s.model_messages.get("claude-sonnet-5"), Some(&1));
    }

    #[test]
    fn empty_input_yields_zero() {
        let s = parse_session_lines(std::iter::empty::<&str>());
        assert_eq!(s.assistant_messages, 0);
        assert_eq!(s.total_tokens(), 0);
    }

    #[test]
    fn history_signal_needs_enough_sessions() {
        let mut h = HistoryStats::default();
        h.sessions = 2;
        assert!(h.signal().is_none());
        h.sessions = 5;
        h.totals.input_tokens = 5 * 500_000; // large sessions
        let sig = h.signal().unwrap();
        assert!(sig.relative_size > 1.0);
    }

    #[test]
    fn averages_are_computed() {
        let mut h = HistoryStats::default();
        h.sessions = 4;
        h.totals.assistant_messages = 40;
        h.totals.input_tokens = 400_000;
        assert_eq!(h.avg_messages_per_session(), 10.0);
        assert_eq!(h.avg_tokens_per_session(), 100_000.0);
    }
}
