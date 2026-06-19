//! Calibration log: decision history for feedback-driven learning.
//!
//! Records every pipeline run as a [`CalibrationEntry`] in a fixed-size
//! ring buffer. The log is consumed by [`SelfKnowledge`](crate::knowledge::SelfKnowledge)
//! to calibrate response patterns, and by [`AttentionScheduler`](crate::attention::AttentionScheduler)
//! to adjust bandwidth over time.
//!
//! ## Design
//!
//! - Fixed window (default 64 entries). Oldest entries are silently evicted.
//! - In-memory only for v0.3.0. Persistent storage is a future concern.
//! - Thread-safe by design: pipeline owns the log, single-threaded access.

use std::collections::VecDeque;

use crate::attention::AttentionCmd;
use crate::budget::DepthLevel;
use crate::core::value::TritValue;
use crate::meta::Domain;

// ── CalibrationEntry ──────────────────────────────────────────────

/// A single pipeline run recorded for calibration purposes.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationEntry {
    /// ID of the scenario that produced this entry.
    pub scenario_id: String,
    /// Domain of the decision.
    pub domain: Domain,
    /// Final trit value after arbitration + SafeFallback + anchor.
    pub result: TritValue,
    /// Final phase after all stages.
    pub phase: f64,
    /// Number of cross-frame interrupts detected.
    pub interrupt_count: usize,
    /// Total pipeline elapsed time in microseconds.
    pub elapsed_us: u64,
    /// Compute depth level used for this run.
    pub depth_level: DepthLevel,
    /// Attention command produced (if any).
    pub attention_cmd: Option<AttentionCmd>,
}

impl CalibrationEntry {
    /// Returns true if this entry represents a "clean" decision
    /// (no interrupts, no SafeFallback, result is computable).
    pub fn is_clean(&self) -> bool {
        self.interrupt_count == 0 && self.result.is_computable()
    }

    /// Returns true if this entry represents a conflicted decision
    /// (interrupts present, result is Hold).
    pub fn is_conflicted(&self) -> bool {
        self.interrupt_count > 0 && self.result == TritValue::Hold
    }
}

// ── CalibrationLog ────────────────────────────────────────────────

/// Fixed-size ring buffer of calibration entries.
///
/// # Window eviction
///
/// When the log exceeds `window_size`, the oldest entry is silently
/// dropped. This prevents unbounded memory growth while retaining
/// recent history for pattern calibration.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationLog {
    entries: VecDeque<CalibrationEntry>,
    window_size: usize,
}

impl CalibrationLog {
    /// Create a new calibration log with the given window size.
    pub fn new(window_size: usize) -> Self {
        CalibrationLog {
            entries: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Record a new entry. Evicts the oldest entry if the window is full.
    pub fn record(&mut self, entry: CalibrationEntry) {
        if self.entries.len() >= self.window_size {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Number of entries currently in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The window size (maximum entries).
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    /// Iterate over entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &CalibrationEntry> {
        self.entries.iter()
    }

    /// Count entries matching a predicate.
    pub fn count_where(&self, predicate: impl Fn(&CalibrationEntry) -> bool) -> usize {
        self.entries.iter().filter(|e| predicate(e)).count()
    }

    /// Get the most recent entry, if any.
    pub fn latest(&self) -> Option<&CalibrationEntry> {
        self.entries.back()
    }

    /// Get the N most recent entries (up to the log size).
    pub fn recent(&self, n: usize) -> Vec<&CalibrationEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    /// Fraction of entries that are "clean" (no interrupts, computable result).
    /// Returns 0.0 if the log is empty.
    pub fn clean_ratio(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        self.count_where(|e| e.is_clean()) as f64 / self.entries.len() as f64
    }

    /// Average phase of all entries. Returns 0.5 if empty.
    pub fn average_phase(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.5;
        }
        self.entries.iter().map(|e| e.phase).sum::<f64>() / self.entries.len() as f64
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for CalibrationLog {
    fn default() -> Self {
        Self::new(64)
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, interrupts: usize, result: TritValue) -> CalibrationEntry {
        CalibrationEntry {
            scenario_id: id.to_string(),
            domain: Domain::General,
            result,
            phase: 0.5,
            interrupt_count: interrupts,
            elapsed_us: 1000,
            depth_level: DepthLevel::Standard,
            attention_cmd: None,
        }
    }

    #[test]
    fn new_log_is_empty() {
        let log = CalibrationLog::new(64);
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn record_adds_entry() {
        let mut log = CalibrationLog::new(64);
        log.record(entry("s1", 0, TritValue::True));
        assert_eq!(log.len(), 1);
        assert_eq!(log.latest().unwrap().scenario_id, "s1");
    }

    #[test]
    fn window_eviction_drops_oldest() {
        let mut log = CalibrationLog::new(3);
        log.record(entry("s1", 0, TritValue::True));
        log.record(entry("s2", 0, TritValue::False));
        log.record(entry("s3", 0, TritValue::Hold));
        log.record(entry("s4", 1, TritValue::Hold)); // s1 should be evicted
        assert_eq!(log.len(), 3);
        assert_eq!(log.latest().unwrap().scenario_id, "s4");
        // Oldest should now be s2
        let first = log.iter().next().unwrap();
        assert_eq!(first.scenario_id, "s2");
    }

    #[test]
    fn clean_ratio_computes_correctly() {
        let mut log = CalibrationLog::new(64);
        log.record(entry("s1", 0, TritValue::True));
        log.record(entry("s2", 2, TritValue::Hold));
        log.record(entry("s3", 0, TritValue::False));
        assert!((log.clean_ratio() - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn clean_ratio_empty_is_zero() {
        let log = CalibrationLog::new(64);
        assert_eq!(log.clean_ratio(), 0.0);
    }

    #[test]
    fn average_phase_empty_is_neutral() {
        let log = CalibrationLog::new(64);
        assert_eq!(log.average_phase(), 0.5);
    }

    #[test]
    fn recent_returns_newest_first() {
        let mut log = CalibrationLog::new(64);
        log.record(entry("s1", 0, TritValue::True));
        log.record(entry("s2", 0, TritValue::False));
        let recent = log.recent(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].scenario_id, "s2");
    }

    #[test]
    fn is_clean_detects_no_interrupts() {
        let e = entry("s", 0, TritValue::True);
        assert!(e.is_clean());
        assert!(!e.is_conflicted());
    }

    #[test]
    fn is_conflicted_detects_interrupts_with_hold() {
        let e = entry("s", 3, TritValue::Hold);
        assert!(e.is_conflicted());
        assert!(!e.is_clean());
    }

    #[test]
    fn clear_empties_log() {
        let mut log = CalibrationLog::new(64);
        log.record(entry("s1", 0, TritValue::True));
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn count_where_filters_correctly() {
        let mut log = CalibrationLog::new(64);
        log.record(entry("s1", 0, TritValue::True));
        log.record(entry("s2", 2, TritValue::Hold));
        log.record(entry("s3", 1, TritValue::Hold));
        assert_eq!(log.count_where(|e| e.is_clean()), 1);
        assert_eq!(log.count_where(|e| e.is_conflicted()), 2);
    }
}
