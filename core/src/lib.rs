//! Core logic for **Claude Usage Tray**.
//!
//! This crate is intentionally free of any GUI/Tauri dependency so that all
//! logic can be unit-tested without a window and without contacting Anthropic.
//!
//! Modules:
//! - [`credentials`] — locate and read the local Claude Code OAuth token.
//! - [`client`] / [`usage`] — query the on-demand usage endpoint and normalize it.
//! - [`models`] — model catalog and relative-cost weighting.
//! - [`estimator`] — heuristic "can my prompt finish?" estimator (pure, deterministic).
//! - [`history`] — parse local Claude Code transcripts for real session statistics.
//! - [`settings`] — persisted user preferences.

pub mod client;
pub mod credentials;
pub mod error;
pub mod estimator;
pub mod history;
pub mod models;
pub mod settings;
pub mod usage;

pub use error::{Error, Result};
